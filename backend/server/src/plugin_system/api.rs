pub mod v1 {
    wasmtime::component::bindgen!({
        async: true,
        world: "eldoria-hub:api/api",
        trappable_imports: true,
        path: "../../plugin-api/wit",
    });
}