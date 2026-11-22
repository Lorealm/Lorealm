//! Raising the open-file limit (`RLIMIT_NOFILE`).

#[derive(Debug, Clone, Copy)]
pub struct FdLimit {
    pub soft: u64,
    pub hard: u64,
    pub requested: u64,
}

impl FdLimit {
    /// True when the request was cut short by the hard limit.
    pub fn capped(&self) -> bool {
        self.soft < self.requested
    }
}

/// Parses a `--max-open-files` value: a number, or `max` for the hard limit.
pub fn parse(value: &str) -> Result<Option<u64>, String> {
    if value.eq_ignore_ascii_case("max") {
        return Ok(None);
    }
    value
        .parse()
        .map(Some)
        .map_err(|_| format!("expected a number or `max`, got `{value}`"))
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // rlim_t isn't u64 on every Unix
pub fn raise(target: Option<u64>) -> std::io::Result<Option<FdLimit>> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a valid, writable rlimit.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let hard = limit.rlim_max as u64;
    let current = limit.rlim_cur as u64;
    let requested = target.unwrap_or(hard);
    let wanted = requested.min(hard).max(current);

    if wanted > current && !set_soft(&mut limit, wanted) {
        // macOS refuses a soft limit above OPEN_MAX even when the hard limit
        // is unlimited.
        let fallback = wanted.min(10_240).max(current);
        if !set_soft(&mut limit, fallback) {
            return Err(std::io::Error::last_os_error());
        }
    }

    // SAFETY: as above.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(Some(FdLimit {
        soft: limit.rlim_cur as u64,
        hard: limit.rlim_max as u64,
        requested,
    }))
}

#[cfg(unix)]
fn set_soft(limit: &mut libc::rlimit, soft: u64) -> bool {
    limit.rlim_cur = soft as libc::rlim_t;
    // SAFETY: `limit` is a valid rlimit with soft <= hard.
    unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, limit) == 0 }
}

#[cfg(not(unix))]
pub fn raise(_target: Option<u64>) -> std::io::Result<Option<FdLimit>> {
    Ok(None)
}

/// Raises the limit and reports the outcome on stderr; failure is not fatal.
pub fn raise_and_report(program: &str, value: &str) {
    let target = match parse(value) {
        Ok(target) => target,
        Err(e) => {
            eprintln!("{program}: --max-open-files: {e}");
            std::process::exit(2);
        }
    };
    match raise(target) {
        Ok(Some(limit)) if limit.capped() => eprintln!(
            "{program}: open-file limit is {} (asked for {}, capped by the hard limit). \
             Raise the hard limit with `ulimit -Hn`, systemd's LimitNOFILE or Docker's --ulimit.",
            limit.soft, limit.requested
        ),
        Ok(Some(limit)) => eprintln!("{program}: open-file limit is {}", limit.soft),
        Ok(None) => {}
        Err(e) => eprintln!("{program}: could not raise the open-file limit: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_and_max() {
        assert_eq!(parse("max"), Ok(None));
        assert_eq!(parse("MAX"), Ok(None));
        assert_eq!(parse("4096"), Ok(Some(4096)));
        assert!(parse("lots").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn never_lowers_the_limit() {
        let before = raise(Some(1)).unwrap().unwrap();
        assert!(before.soft >= 1);
        let after = raise(Some(1)).unwrap().unwrap();
        assert_eq!(before.soft, after.soft);
    }
}
