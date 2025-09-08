//! CLI commands for plugin management
use crate::error::Result as RfgrepResult;
use crate::plugin_system::EnhancedPluginManager;
use colored::Colorize;
use std::sync::Arc;

/// Plugin management CLI commands
pub struct PluginCli {
    manager: Arc<EnhancedPluginManager>,
}

impl PluginCli {
    pub fn new(manager: Arc<EnhancedPluginManager>) -> Self {
        Self { manager }
    }

    /// List all available plugins
    pub async fn list_plugins(&self) -> RfgrepResult<()> {
        let plugins = self.manager.list_plugins().await;

        if plugins.is_empty() {
            println!("{}", "No plugins registered".yellow());
            return Ok(());
        }

        println!("{}", "Available Plugins:".green().bold());
        println!("{}", "==================".green());

        for plugin in plugins {
            let status = if plugin.enabled {
                "ENABLED".green()
            } else {
                "DISABLED".red()
            };

            let streaming = if plugin.supports_streaming {
                " [STREAMING]".blue()
            } else {
                "".clear()
            };

            println!("\n{} {} {}", plugin.name.bold(), status, streaming);
            println!("  Version: {}", plugin.version);
            println!("  Description: {}", plugin.description);
            println!("  Priority: {}", plugin.priority);
            println!("  Extensions: {}", plugin.supported_extensions.join(", "));
        }

        Ok(())
    }

    /// Show plugin statistics
    pub async fn show_stats(&self) -> RfgrepResult<()> {
        let stats = self.manager.get_plugin_stats().await;

        println!("{}", "Plugin Statistics:".green().bold());
        println!("{}", "==================".green());
        println!("Total Plugins: {}", stats.total_plugins);
        println!("Enabled: {}", format!("{}", stats.enabled_plugins).green());
        println!("Disabled: {}", format!("{}", stats.disabled_plugins).red());
        println!(
            "Streaming Support: {}",
            format!("{}", stats.streaming_plugins).blue()
        );

        Ok(())
    }

    /// Enable a plugin
    pub async fn enable_plugin(&self, name: &str) -> RfgrepResult<()> {
        if let Some(mut config) = self.manager.get_plugin_config(name).await {
            config.enabled = true;
            self.manager.update_plugin_config(name, config).await?;
            println!("{}", format!("Plugin '{}' enabled", name).green());
        } else {
            println!("{}", format!("Plugin '{}' not found", name).red());
        }
        Ok(())
    }

    /// Disable a plugin
    pub async fn disable_plugin(&self, name: &str) -> RfgrepResult<()> {
        if let Some(mut config) = self.manager.get_plugin_config(name).await {
            config.enabled = false;
            self.manager.update_plugin_config(name, config).await?;
            println!("{}", format!("Plugin '{}' disabled", name).yellow());
        } else {
            println!("{}", format!("Plugin '{}' not found", name).red());
        }
        Ok(())
    }

    /// Show detailed plugin information
    pub async fn show_plugin_info(&self, name: &str) -> RfgrepResult<()> {
        let plugins = self.manager.list_plugins().await;

        if let Some(plugin) = plugins.iter().find(|p| p.name == name) {
            println!("{}", format!("Plugin: {}", plugin.name).green().bold());
            println!("{}", "=".repeat(plugin.name.len() + 8).green());
            println!("Version: {}", plugin.version);
            println!("Description: {}", plugin.description);
            println!(
                "Status: {}",
                if plugin.enabled {
                    "ENABLED".green()
                } else {
                    "DISABLED".red()
                }
            );
            println!("Priority: {}", plugin.priority);
            println!(
                "Streaming Support: {}",
                if plugin.supports_streaming {
                    "Yes".green()
                } else {
                    "No".red()
                }
            );
            println!(
                "Supported Extensions: {}",
                plugin.supported_extensions.join(", ")
            );

            // Show configuration if available
            if let Some(config) = self.manager.get_plugin_config(name).await {
                if !config.settings.is_empty() {
                    println!("\nConfiguration:");
                    for (key, value) in &config.settings {
                        println!("  {}: {}", key, value);
                    }
                }
            }
        } else {
            println!("{}", format!("Plugin '{}' not found", name).red());
        }

        Ok(())
    }

    /// Set plugin priority
    pub async fn set_priority(&self, name: &str, priority: u32) -> RfgrepResult<()> {
        if let Some(mut config) = self.manager.get_plugin_config(name).await {
            config.priority = priority;
            self.manager.update_plugin_config(name, config).await?;
            println!(
                "{}",
                format!("Plugin '{}' priority set to {}", name, priority).green()
            );
        } else {
            println!("{}", format!("Plugin '{}' not found", name).red());
        }
        Ok(())
    }

    /// Show plugin configuration options
    pub async fn show_config_options(&self, name: &str) -> RfgrepResult<()> {
        let plugins = self.manager.list_plugins().await;

        if let Some(plugin) = plugins.iter().find(|p| p.name == name) {
            println!(
                "{}",
                format!("Configuration Options for '{}':", plugin.name)
                    .green()
                    .bold()
            );
            println!("{}", "=".repeat(40).green());

            // This would need to be implemented in the plugin manager
            // to get actual configuration options from the plugin
            println!("Configuration options not yet implemented for this plugin.");
        } else {
            println!("{}", format!("Plugin '{}' not found", name).red());
        }

        Ok(())
    }

    /// Test a plugin with a specific file
    pub async fn test_plugin(
        &self,
        name: &str,
        file_path: &str,
        pattern: &str,
    ) -> RfgrepResult<()> {
        use std::path::Path;

        let path = Path::new(file_path);
        if !path.exists() {
            println!("{}", format!("File '{}' does not exist", file_path).red());
            return Ok(());
        }

        println!(
            "{}",
            format!(
                "Testing plugin '{}' on file '{}' with pattern '{}'",
                name, file_path, pattern
            )
            .green()
            .bold()
        );
        println!("{}", "=".repeat(60).green());

        match self.manager.search_file(path, pattern).await {
            Ok(matches) => {
                if matches.is_empty() {
                    println!("{}", "No matches found".yellow());
                } else {
                    println!("{}", format!("Found {} matches:", matches.len()).green());
                    for (i, m) in matches.iter().enumerate() {
                        println!("  {}. Line {}: {}", i + 1, m.line_number, m.line);
                    }
                }
            }
            Err(e) => {
                println!("{}", format!("Error: {}", e).red());
            }
        }

        Ok(())
    }

    /// Show help for plugin commands
    pub fn show_help(&self) {
        println!("{}", "Plugin Management Commands:".green().bold());
        println!("{}", "===========================".green());
        println!("list                    - List all available plugins");
        println!("stats                  - Show plugin statistics");
        println!("info <name>            - Show detailed plugin information");
        println!("enable <name>          - Enable a plugin");
        println!("disable <name>         - Disable a plugin");
        println!("priority <name> <num>  - Set plugin priority");
        println!("config <name>          - Show plugin configuration options");
        println!("test <name> <file> <pattern> - Test plugin with specific file");
        println!("help                   - Show this help message");
    }
}
