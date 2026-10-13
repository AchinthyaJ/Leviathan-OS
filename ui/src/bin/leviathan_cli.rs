use leviathan_ui::package::PackageManager;
use std::env;

static BANNER: &str = r#"========================================================================
    __    _______ _    __ ____ ___  ______ __  __ ___    _   __
   / /   / ____/| |  / //  _//   |/_  __// / / //   |  / | / /
  / /   / __/   | | / / / / / /| | / /  / /_/ // /| | /  |/ / 
 / /___/ /___   | |/ /_/ / / ___ |/ /  / __  // ___ |/ /|  /  
/_____/_____/   |___//___//_/  |_|/_/  /_/ /_//_/  |_/_/ |_/   

                   --- L E V I A T H A N ---
             Unified Linux Userspace & Package Management
========================================================================"#;

fn print_usage() {
    println!("{}", BANNER);
    println!("\nUsage: leviathan <command> [options]\n");
    println!("Commands:");
    println!("  package search <term>    Search for packages in repositories");
    println!("  package info <name>      Display detailed package metadata");
    println!("  package install <name>   Install a package via APT/dpkg");
    println!("  package remove <name>    Remove an installed package");
    println!("  package update           Update repository package indices");
    println!("  package upgrade          Upgrade all upgradable packages");
    println!("  package list             List all currently installed packages");
    println!("  network                  Display network status and IP addresses");
    println!("  version                  Show Leviathan OS release version");
    println!("  help                     Show this usage information\n");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "version" | "--version" | "-v" => {
            println!("Leviathan OS v0.2.0 (x86_64-musl-linux)");
            println!("Linux Kernel 6.8.0-generic | APT 2.8.3 / dpkg 1.22.6");
        }
        "network" | "net" => {
            let _ = std::process::Command::new("/bin/net").status();
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "package" | "pkg" => {
            if args.len() < 3 {
                println!("Error: Missing package subcommand.");
                println!("Run 'leviathan help' for usage details.");
                std::process::exit(1);
            }

            match args[2].as_str() {
                "list" => {
                    let pkgs = PackageManager::list_installed();
                    println!("{:<24} {:<18} {:<12} {}", "PACKAGE", "VERSION", "SIZE", "DESCRIPTION");
                    println!("{:-<24} {:-<18} {:-<12} {:-<35}", "", "", "", "");
                    for p in pkgs {
                        let size_str = if p.installed_size_kb > 1024 {
                            format!("{:.1} MB", p.installed_size_kb as f64 / 1024.0)
                        } else {
                            format!("{} KB", p.installed_size_kb)
                        };
                        println!("{:<24} {:<18} {:<12} {}", p.name, p.version, size_str, p.description);
                    }
                }
                "search" => {
                    let term = if args.len() >= 4 { &args[3] } else { "" };
                    println!("Searching Leviathan packages for '{}'...\n", term);
                    let results = PackageManager::search(term);
                    if results.is_empty() {
                        println!("No packages matching '{}' found.", term);
                    } else {
                        println!("{:<20} {:<14} {:<12} {}", "PACKAGE", "VERSION", "STATUS", "DESCRIPTION");
                        println!("{:-<20} {:-<14} {:-<12} {:-<35}", "", "", "", "");
                        for p in results {
                            let status = if p.is_installed { "[installed]" } else { "[available]" };
                            println!("{:<20} {:<14} {:<12} {}", p.name, p.version, status, p.description);
                        }
                    }
                }
                "info" => {
                    if args.len() < 4 {
                        eprintln!("Error: 'leviathan package info <name>' requires a package name.");
                        std::process::exit(1);
                    }
                    let name = &args[3];
                    if let Some(d) = PackageManager::info(name) {
                        println!("Package:      {}", d.info.name);
                        println!("Version:      {}", d.info.version);
                        println!("Status:       {}", d.status);
                        println!("Architecture: {}", d.architecture);
                        println!("Category:     {}", d.info.category);
                        println!("Section:      {}", d.info.section);
                        println!("Maintainer:   {}", d.maintainer);
                        println!("Dependencies: {}", d.dependencies.join(", "));
                        println!("Description:  {}", d.info.description);
                        println!("Homepage:     {}", d.homepage);
                    } else {
                        eprintln!("Package '{}' not found in database or catalog.", name);
                        std::process::exit(1);
                    }
                }
                "update" => {
                    println!("Updating Leviathan repository indices...");
                    match PackageManager::update(|line| println!("  {}", line)) {
                        Ok(()) => println!("\nRepositories successfully synchronized."),
                        Err(e) => {
                            eprintln!("\nError updating repositories: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                "install" => {
                    if args.len() < 4 {
                        eprintln!("Error: 'leviathan package install <name>' requires a package name.");
                        std::process::exit(1);
                    }
                    let name = &args[3];
                    println!("Installing package '{}'...", name);
                    match PackageManager::install(name, |line| println!("  {}", line)) {
                        Ok(()) => println!("\nSuccessfully installed '{}'.", name),
                        Err(e) => {
                            eprintln!("\nFailed to install '{}': {}", name, e);
                            std::process::exit(1);
                        }
                    }
                }
                "remove" => {
                    if args.len() < 4 {
                        eprintln!("Error: 'leviathan package remove <name>' requires a package name.");
                        std::process::exit(1);
                    }
                    let name = &args[3];
                    println!("Removing package '{}'...", name);
                    match PackageManager::remove(name, |line| println!("  {}", line)) {
                        Ok(()) => println!("\nSuccessfully removed '{}'.", name),
                        Err(e) => {
                            eprintln!("\nFailed to remove '{}': {}", name, e);
                            std::process::exit(1);
                        }
                    }
                }
                "upgrade" => {
                    println!("Upgrading system packages...");
                    match PackageManager::update(|line| println!("  {}", line)) {
                        Ok(()) => println!("\nSystem upgrade completed."),
                        Err(e) => {
                            eprintln!("\nUpgrade failed: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                other => {
                    eprintln!("Unknown package subcommand: '{}'. Run 'leviathan help' for usage.", other);
                    std::process::exit(1);
                }
            }
        }
        cmd => {
            eprintln!("Unknown command '{}'. Run 'leviathan help' for usage.", cmd);
            std::process::exit(1);
        }
    }
}
