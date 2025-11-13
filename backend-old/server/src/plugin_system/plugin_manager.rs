use std::{cell::RefCell, path::PathBuf, sync::{Arc, Mutex}};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use wasmtime::{component::{Component, Instance, Linker}, AsContextMut, Config, Engine, Store};
use wasmtime_wasi::{IoView, ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

use crate::{file, plugin_system::{permission_manager::PermissionState, plugin_permissions::permissions_to_json}};
use super::{api::{self, v1::{eldoria_hub::api::log::LogLevel, Api}}, loader, permission_manager::{PermissionManager, PersistentResource}, plugin::Plugin, plugin_helpers::get_typed_function, plugin_permissions::{ContextPermission, HttpPermission, LogPermission, PluginPermission}};

thread_local! {
    static ACTIVE_PLUGINS: RefCell<Vec<PluginInstance>> = RefCell::new(Vec::new());
    static PERMISSION_MANAGER: RefCell<PermissionManager> = RefCell::new(PermissionManager::new());
}

lazy_static::lazy_static! {
    static ref COMPONENT_QUEUE: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
}

fn get_active_plugin() -> Option<PluginInstance> {
    ACTIVE_PLUGINS.with(|stack| stack.borrow().last().cloned())
}

#[derive(Clone)]
pub struct PluginInstance {
    plugin: Plugin,
    instance: Instance,
    component: Component
}

impl PluginInstance {
    fn new(plugin: Plugin, instance: Instance, component: Component) -> Self {
        Self {
            plugin,
            instance,
            component
        }
    }

    pub fn plugin(&self) -> &Plugin {
        &self.plugin
    }

    pub fn instance(&self) -> &Instance {
        &self.instance
    }

    pub fn component(&self) -> &Component {
        &self.component
    }
}

pub struct PluginManager {
    plugins: Vec<Plugin>,
    engine: Engine,
    store: Store<WasmState>
}

impl PluginManager {
    pub fn new() -> Self {
        let mut config = Config::new();  
        config.async_support(true);  
        config.wasm_component_model(true);  
        config.debug_info(true);

        let engine = Engine::new(&config).unwrap();
        let store = Store::new(&engine, WasmState {
            ctx: WasiCtxBuilder::new().build(),
            table: ResourceTable::new()
        });

        Self {
            plugins: Vec::new(),
            engine: engine,
            store: store
        }
    }

    fn push_active_plugin(&mut self, plugin: PluginInstance) {
        ACTIVE_PLUGINS.with(|stack| stack.borrow_mut().push(plugin));
    }

    fn pop_active_plugin(&mut self) {
        ACTIVE_PLUGINS.with(|stack| {
            let mut stack_mut = stack.borrow_mut();
            if !stack_mut.is_empty() {
                stack_mut.pop();
            }
        });
    }

    fn get_active_plugin(&self) -> Option<PluginInstance> {
        ACTIVE_PLUGINS.with(|stack| stack.borrow().last().cloned())
    }

    fn load_plugins(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Loading plugins");
        let path = file::get_path("plugins")?;
        self.plugins = loader::parse_plugins(&path);
        Ok(())
    }

    async fn instantiate_plugin(&mut self, plugin: Plugin) -> Result<PluginInstance, Box<dyn std::error::Error>> {
        log::info!("Instantiating plugin: {}", plugin.name);

        let mut linker = Linker::<WasmState>::new(&self.engine);

        Api::add_to_linker(&mut linker, |s: &mut WasmState| s).expect("Failed to add API to linker");
        wasmtime_wasi::add_to_linker_async(&mut linker).expect("Failed to add WASI to linker");

        let component = Component::from_file(&self.engine, &plugin.binary_path)?;
        let instance = linker.instantiate_async(&mut self.store, &component).await.expect("Failed to instantiate plugin");

        Ok(PluginInstance::new(plugin, instance, component))
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing plugin manager");

        self.load_plugins()?;

        // First, load and instantiate all plugins
        let mut loaded_instances = Vec::new();
        for plugin in self.plugins.clone() {
            let instance = self.instantiate_plugin(plugin.clone()).await?;
            loaded_instances.push(instance);
        }

        // Then, process events for each plugin
        for instance in &loaded_instances {
            self.push_active_plugin(instance.clone());
            self.on_plugin_loaded(&instance).await?;
            self.pop_active_plugin();

            // Check permissions separately
            let resolved = PERMISSION_MANAGER.with(|pm| {
                let mut manager = pm.borrow_mut();
                match manager.add_plugin(instance.plugin().clone()) {
                    Ok(state) => state == PermissionState::Granted,
                    Err(_) => false,
                }
            });

            if resolved {
                self.push_active_plugin(instance.clone());
                self.on_permissions_granted(&instance).await?;
            }
        }

        // Finally, add plugins to your managed list
        self.plugins = loaded_instances.iter().map(|i| i.plugin.clone()).collect();

        Ok(())
    }

    pub fn prepare_permission_requests(&self) -> Vec<PluginPermissionRequest> {
        let requests = PERMISSION_MANAGER.with(|pm| {
            let manager = pm.borrow();
            let pending = manager.get_pending_plugins();
            pending.iter().map(|p| PluginPermissionRequest {
                plugin_name: p.name.clone(),
                plugin_id: p.id,
                permissions: permissions_to_json(&p.permissions())
            }).collect()
        });

        requests
    }

    pub fn get_custom_components(&self) -> Vec<String> {
        let components = COMPONENT_QUEUE.lock().unwrap();
        
        return components.clone();
    }

    pub(crate) fn submit_permission_grants(&self, plugin_id: Uuid, granted_permissions: Vec<PluginPermission>) {
        PERMISSION_MANAGER.with(|pm| -> () {
            let mut manager = pm.borrow_mut();
            manager.resolve_pending_plugin(plugin_id, granted_permissions).unwrap();
            
            match manager.save() {
                Ok(_) => (),
                Err(e) => log::warn!("Failed to save permissions: {}", e),
            };
        });
    }

    pub fn get_plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

trait PluginEvents {
    async fn on_plugin_loaded(&mut self, instance: &PluginInstance) -> Result<(), Box<dyn std::error::Error>>;
    async fn on_permissions_granted(&mut self, instance: &PluginInstance) -> Result<(), Box<dyn std::error::Error>>;
}

// Move call_function out of the trait
async fn call_function<T>(
    instance: &PluginInstance, 
    path: &str, 
    mut store: impl AsContextMut<Data = T>
) -> Result<(), Box<dyn std::error::Error>>
where T: Send
{
    let func = get_typed_function::<(), ()>(instance, path, &mut store)?;
    match func.call_async(&mut store, ()).await {
        Ok(_) => func.post_return_async(&mut store).await?,
        Err(e) => log::warn!("Failed to call function: {}", e),
    }
    Ok(())
}

impl PluginEvents for PluginManager {
    async fn on_plugin_loaded(&mut self, instance: &PluginInstance) -> Result<(), Box<dyn std::error::Error>> {
        let store = &mut self.store;
        call_function(instance, "load-events:on-plugin-loaded", store).await
    }
    
    async fn on_permissions_granted(&mut self, instance: &PluginInstance) -> Result<(), Box<dyn std::error::Error>> {
        let store = &mut self.store;
        call_function(instance, "load-events:on-permissions-granted", store).await
    }
}

struct WasmState {  
    ctx: WasiCtx,  
    table: ResourceTable,  
}  

impl IoView for WasmState {
    fn table(&mut self) -> &mut ResourceTable { &mut self.table }
}

impl WasiView for WasmState {  
    fn ctx(&mut self) -> &mut WasiCtx { &mut self.ctx }  
}

fn validate_access(permission: PluginPermission) -> Result<PluginInstance, wasmtime::Error> {
    let plugin = get_active_plugin();

    if plugin.is_none() {
        log::debug!("No active plugin");
        return Err(anyhow::anyhow!("No active plugin"));
    }

    let instance = plugin.unwrap();
    if PERMISSION_MANAGER.with(|pm| {
        let manager = pm.borrow();
        !manager.has_explicit_permission(&instance.plugin, &permission)
    }) {
        log::warn!("Plugin {} tried to call a function, but doesn't have required permissions ({})", &instance.plugin.name, permission.to_str());
        return Err(anyhow::anyhow!("Plugin {} tried to call a function, but doesn't have required permissions ({})", &instance.plugin.name, permission.to_str()));
    }

    Ok(instance)
}

impl api::v1::eldoria_hub::api::log::Host for WasmState {
    async fn log(&mut self, content: String, log_level: LogLevel) -> Result<(), wasmtime::Error> {
        let instance = validate_access(PluginPermission::Log(LogPermission::Write))?;

        log::debug!("Plugin {} called api::log", &instance.plugin.name);

        match log_level {
            LogLevel::Trace => Ok(log::trace!("[{}] {} [{}]", instance.plugin.name, content, instance.plugin.id())),
            LogLevel::Debug => Ok(log::debug!("[{}] {} [{}]", instance.plugin.name, content, instance.plugin.id())),
            LogLevel::Info => Ok(log::info!("[{}] {}", instance.plugin.name, content)),
            LogLevel::Warning => Ok(log::warn!("[{}] {}", instance.plugin.name, content)),
            LogLevel::Error => Ok(log::error!("[{}] {}", instance.plugin.name, content)),
            LogLevel::Critical => Ok(log::error!("CRITCAL ERROR: [{}] {}", instance.plugin.name, content)),
        }
    }
}

impl api::v1::ApiImports for WasmState {
    async fn register_component(&mut self, name: String) -> Result<(), wasmtime::Error> {
        let instance = validate_access(PluginPermission::Log(LogPermission::Write))?;
        
        let binary_path = PathBuf::from(instance.plugin.binary_path.clone());
        let plugin_dir = binary_path.parent().unwrap();
        let component_path = plugin_dir.join("components").join(&name);

        if !component_path.exists() {
            log::warn!("Component {} does not exist", name);
            return Err(anyhow::anyhow!("Component {} does not exist", name));
        }

        log::info!("Registering component {}", name);
        COMPONENT_QUEUE.lock().unwrap().push(component_path.to_str().unwrap().to_string());

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginPermissionRequest {
    pub plugin_name: String,
    pub plugin_id: Uuid,
    pub permissions: Value
}

impl PluginPermissionRequest {
    /// Converts the JSON permissions structure into a vector of PluginPermission
    pub fn to_permissions_list(&self) -> Vec<PluginPermission> {
        let mut permissions = Vec::new();
        
        // Extract log permissions
        if let Some(log) = self.permissions.get("log") {
            if let Some(true) = log.get("read").and_then(|v| v.as_bool()) {
                permissions.push(PluginPermission::Log(LogPermission::Read));
            }
            if let Some(true) = log.get("write").and_then(|v| v.as_bool()) {
                permissions.push(PluginPermission::Log(LogPermission::Write));
            }
        }
        
        // Extract context permissions
        if let Some(context) = self.permissions.get("context") {
            if let Some(true) = context.get("read").and_then(|v| v.as_bool()) {
                permissions.push(PluginPermission::Context(ContextPermission::Read));
            }
            if let Some(true) = context.get("write").and_then(|v| v.as_bool()) {
                permissions.push(PluginPermission::Context(ContextPermission::Write));
            }
        }
        
        // Extract http permissions
        if let Some(http) = self.permissions.get("http") {
            if let Some(true) = http.get("get").and_then(|v| v.as_bool()) {
                permissions.push(PluginPermission::Http(HttpPermission::Get));
            }
            if let Some(true) = http.get("post").and_then(|v| v.as_bool()) {
                permissions.push(PluginPermission::Http(HttpPermission::Post));
            }
        }
        
        permissions
    }
}