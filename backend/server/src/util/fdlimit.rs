//! Raising the open-file limit (`RLIMIT_NOFILE`) at startup.

use crate::config::OpenFileLimit;

pub fn apply(limit: OpenFileLimit) {
    let target = match limit {
        OpenFileLimit::System => return,
        OpenFileLimit::Max => None,
        OpenFileLimit::Count(n) => Some(n),
    };
    match raise(target) {
        Ok(Some(outcome)) if outcome.soft < outcome.requested => tracing::warn!(
            "open-file limit is {} (asked for {}, capped by the hard limit); raise the hard \
             limit with systemd's LimitNOFILE, Docker's --ulimit nofile or `ulimit -Hn`",
            outcome.soft,
            outcome.requested
        ),
        Ok(Some(outcome)) => tracing::info!("open-file limit is {}", outcome.soft),
        Ok(None) => {}
        Err(e) => tracing::warn!("could not raise the open-file limit: {e}"),
    }
}

struct Outcome {
    soft: u64,
    requested: u64,
}

#[cfg(unix)]
#[allow(clippy::unnecessary_cast)] // rlim_t isn't u64 on every Unix
fn raise(target: Option<u64>) -> std::io::Result<Option<Outcome>> {
    let mut limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    // SAFETY: `limit` is a valid, writable rlimit.
    if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut limit) } != 0 {
        return Err(std::io::Error::last_os_error());
    }

    let (current, hard) = (limit.rlim_cur as u64, limit.rlim_max as u64);
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
    Ok(Some(Outcome {
        soft: limit.rlim_cur as u64,
        requested,
    }))
}

#[cfg(unix)]
fn set_soft(limit: &mut libc::rlimit, soft: u64) -> bool {
    limit.rlim_cur = soft as libc::rlim_t;
    // SAFETY: `limit` is a valid rlimit whose soft value is at most its hard one.
    unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, limit) == 0 }
}

#[cfg(not(unix))]
fn raise(_target: Option<u64>) -> std::io::Result<Option<Outcome>> {
    Ok(None)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn never_lowers_the_limit() {
        let before = raise(Some(1)).unwrap().unwrap();
        let after = raise(Some(1)).unwrap().unwrap();
        assert!(before.soft >= 1);
        assert_eq!(before.soft, after.soft);
    }
}
