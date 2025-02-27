extern crate inflector;

mod content;
mod utils;
mod plugins;
mod qsync;

use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;

use console::Term;
use dialoguer::{Input, MultiSelect, Select, theme::ColorfulTheme};
use content::project;
use utils::{fs, logger};
use crate::project::CreationOptions;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackendFramework {
    ActixWeb, Poem
}

#[derive(StructOpt, Debug)]
#[structopt(name = "create-rust-app")]
struct UnknownOpt {
    #[structopt(name = "name")]
    target: Option<String>,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "create-rust-app")]
struct CreateOpt {
    #[structopt(name = "name")]
    target: String,
}

#[derive(StructOpt, Debug)]
#[structopt(name = "create-rust-app")]
struct UpdateOpt {
    #[structopt(
        short = "qsync",
        long,
        name = "query-sync",
        help = "Generate react-query hooks for frontend."
    )]
    query_sync: bool,
}

/// CREATE RUST APP
///
/// A MODERN WAY TO BOOTSTRAP A RUST+REACT APP IN A SINGLE COMMAND
fn main() -> Result<()> {
    let unknown_opts = UnknownOpt::from_args();

    let mut current_dir: PathBuf = fs::get_current_working_directory()?;

    if unknown_opts.target.is_some() {
        current_dir = PathBuf::from(unknown_opts.target.unwrap());

        if current_dir.exists() {
            logger::error(
                &format!(
                    "Cannot create a project: {:#?} already exists.",
                    &current_dir
                )
                .to_string(),
            );
            return Ok(());
        }
    }

    let is_rust_project_directory = current_dir.exists() && fs::is_rust_project(&current_dir)?;

    if !is_rust_project_directory {
        let create_opts = CreateOpt::from_args();

        logger::message("Select a rust backend framework to use:");
        logger::message("Use UP/DOWN arrows to navigate and SPACE or ENTER to confirm.");
        let items = vec!["actix-web", "poem"];
        let selection = Select::with_theme(&ColorfulTheme::default())
                .items(&items)
                .default(0)
                .interact_on_opt(&Term::stderr())?;

        let backend_framework: BackendFramework = match selection {
            Some(0) => BackendFramework::ActixWeb,
            Some(1) => BackendFramework::Poem,
            _ => panic!("Fatal: Unknown backend framework specified.")
        };

        // let valid_frameworks = vec!["actix_web", "poem"];
        //
        // if !valid_frameworks.contains(&backend_framework.as_str()) {
        //     panic!("Invalid framework given. Choose one of: {:#?}", valid_frameworks);
        // }

        let project_name = create_opts.target;

        if project_name.len() == 0 {
            logger::error("Please provide a project name");

            return Ok(());
        }

        logger::message("Please select plugins for your new project:");
        logger::message("Use UP/DOWN arrows to navigate, SPACE to enable/disable a plugin, and ENTER to confirm.");

        let items = vec![
            "Authentication Plugin: local email-based authentication",
            "Container Plugin: dockerize your app",
            "Development Plugin: adds dev warnings and an admin portal",
            "Storage Plugin: adds S3 file storage capabilities"
        ];
        let chosen: Vec<usize> = MultiSelect::with_theme(&ColorfulTheme::default())
            .items(&items)
            .defaults(&[true, true, true, true])
            .interact()?;

        let add_plugin_auth = chosen.iter().position(|x| *x == 0).is_some();
        let add_plugin_container = chosen.iter().position(|x| *x == 1).is_some();
        let add_plugin_dev = chosen.iter().position(|x| *x == 2).is_some();
        let add_plugin_storage = chosen.iter().position(|x| *x == 3).is_some();

        let mut features: Vec<String> = vec!();
        if add_plugin_dev { features.push("plugin_dev".to_string()); }
        if add_plugin_auth { features.push("plugin_auth".to_string()); }
        if add_plugin_container { features.push("plugin_container".to_string()); }
        if add_plugin_storage { features.push("plugin_storage".to_string()); }
        features.push(match backend_framework {
            BackendFramework::ActixWeb => "backend_actix-web".to_string(),
            BackendFramework::Poem => "backend_poem".to_string()
        });

        project::create(project_name.as_ref(), CreationOptions {
            cra_enabled_features: features,
            backend_framework
        })?;

        let mut project_dir = PathBuf::from(".");
        project_dir.push(project_name);
        // !
        std::env::set_current_dir(project_dir.clone()).expect(&format!("Unable to change into {:#?}", project_dir.clone()));

        //
        // Note: we're in the project dir from here on out
        //

        let install_config = plugins::InstallConfig {
            project_dir: PathBuf::from("."),
            backend_framework
        };

        if add_plugin_auth { plugins::install(plugins::auth::Auth {}, install_config.clone())?; }
        if add_plugin_container { plugins::install(plugins::container::Container {}, install_config.clone())?; }
        if add_plugin_dev { plugins::install(plugins::dev::Dev {}, install_config.clone())?; }
        if add_plugin_storage { plugins::install(plugins::storage::Storage {}, install_config.clone())?; }

        // cd into project dir and make a copy of the env file
        let example_env_file = PathBuf::from("./.env.example");
        let env_file = PathBuf::from("./.env");

        let contents = std::fs::read_to_string(example_env_file)
            .expect("Error: Tried to read .env.example contents but an error occurred");
        std::fs::write(env_file, contents)?;
        logger::add_file_msg(".env");

        logger::project_created_msg(project_dir);
    } else {
        let current_dir: PathBuf = fs::get_current_working_directory()?;

        if !current_dir.exists() {
            println!("Fatal: the current directory doesn't exist. This shouldn't be possible.");
            return Ok(());
        }

        if !fs::is_rust_project(&current_dir)? {
            // TODO: determine if the current directory is a create-rust-app project.
            println!("Fatal: the current directory is not a rust project.");
            return Ok(());
        }

        // println!("It looks like you ran `create-rust-app` without a [name] argument in a rust project directory.");
        // println!("This functionality has been temporarily disabled in v3 due to our migration to the poem framework. There are plans to support multiple backend frameworks in the future (specifically: actix_web, rocket, axum, warp, and poem).");
        // println!("\nIf you were trying to create a rust app, include the name argument like so:\n\t{}", style("create-rust-app <project_name>").cyan());
        // return Ok(());

        let update_opts = UpdateOpt::from_args();
        let items = vec!["Generate react-query hooks (beta)", "Add a model & service (beta)", "Cancel"];

        let selection = if update_opts.query_sync {
            Some(0)
        } else {
            Select::with_theme(&ColorfulTheme::default())
                .items(&items)
                .default(0)
                .interact_on_opt(&Term::stderr())?
        };

        match selection {
            Some(index) => {
                match index {
                    0 => {
                        qsync::process(
                            vec![PathBuf::from("backend/services")],
                            PathBuf::from("frontend/src/api.generated.ts"),
                            false,
                        );
                    }
                    1 => {
                        // Add resource
                        let resource_name: String = Input::new()
                            .with_prompt("Resource name")
                            .default("".into())
                            .interact_text()?;

                        if resource_name.len() == 0 {
                            return Ok(());
                        }

                        logger::message("Which backend framework are you using?");
                        logger::message("Use UP/DOWN arrows to navigate and SPACE or ENTER to confirm.");
                        let items = vec!["actix_web", "poem"];
                        let selection = Select::with_theme(&ColorfulTheme::default())
                            .items(&items)
                            .default(0)
                            .interact_on_opt(&Term::stderr())?;

                        let backend_framework: BackendFramework = match selection {
                            Some(0) => BackendFramework::ActixWeb,
                            Some(1) => BackendFramework::Poem,
                            _ => panic!("Fatal: Unknown backend framework specified.")
                        };
                        project::create_resource(backend_framework, resource_name.as_ref())?;
                        std::process::exit(0);
                    }
                    2 => return Ok(()),
                    _ => {
                        logger::error("Not implemented");
                        std::process::exit(1);
                    }
                }
            }
            None => {}
        }
    }

    Ok(())
}
