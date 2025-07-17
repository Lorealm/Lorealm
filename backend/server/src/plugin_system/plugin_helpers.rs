use wasmtime::{component::{Component, ComponentExportIndex, ComponentNamedList, Func, Instance, Lift, Lower, TypedFunc}, AsContextMut};

use super::plugin_manager::PluginInstance;

fn parse_path(path: &str) -> (String, String) {
    let (interface, function_name) = path.split_once(':').unwrap();
    (format!("eldoria-hub:api/{}", interface), function_name.to_string())
}

fn resolve_function_index<'a>(
    plugin_instance: &'a PluginInstance,
    path: &str
) -> Result<(&'a Component, &'a Instance, ComponentExportIndex), Box<dyn std::error::Error>> {
    let (path, function_name) = parse_path(path);
    let component = &plugin_instance.component();
    let instance = &plugin_instance.instance();

    let instance_index = component.export_index(None, &path)
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Instance not found")))?;

    let func_index = component.export_index(Some(&instance_index.1), &function_name)
        .ok_or_else(|| Box::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Function not found")))?;

    Ok((component, instance, func_index.1))
}

pub fn get_typed_function<Params, Results>(
    plugin_instance: &PluginInstance,
    path: &str,
    mut store: impl AsContextMut
) -> Result<TypedFunc<Params, Results>, Box<dyn std::error::Error>>
where
    Params: ComponentNamedList + Lower,
    Results: ComponentNamedList + Lift,
{
    let (_, instance, func_index) = resolve_function_index(plugin_instance, path)?;
    Ok(instance.get_typed_func::<Params, Results>(&mut store, func_index).expect("Should be able to retrieve function"))
}

pub fn get_function(
    plugin_instance: &PluginInstance,
    path: &str,
    mut store: impl AsContextMut
) -> Result<Func, Box<dyn std::error::Error>> {
    let (_, instance, func_index) = resolve_function_index(plugin_instance, path)?;
    Ok(instance.get_func(&mut store, func_index).expect("Should be able to retrieve function"))
}