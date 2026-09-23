wit_bindgen::generate!({
    path: "../../../wit",
    world: "plugin",
});

struct Hello;

impl Guest for Hello {
    fn init() -> Result<(), String> {
        Ok(())
    }
}

export!(Hello);
