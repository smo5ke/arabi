use anyhow::{Result, Context};
use std::fs;
use std::env;
use rustyline::DefaultEditor;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const VERSION: &str = "0.1.0";

fn print_help() {
    println!("\x1b[1;36m═══════════════════════════════════════\x1b[0m");
    println!("\x1b[1;36m       عَرَبِي — لغة البرمجة العربية\x1b[0m");
    println!("\x1b[1;36m═══════════════════════════════════════\x1b[0m");
    println!();
    println!("\x1b[1;33mالاستخدام:\x1b[0m");
    println!("  عربي <ملف>              تشغيل ملف");
    println!("  عربي run <ملف>          تشغيل ملف (بديل)");
    println!("  عربي -d <ملف>           تشغيل مع تفاصيل التجميع");
    println!("  عربي                   REPL تفاعلي");
    println!("  عربي --help            عرض المساعدة");
    println!("  عربي --version         عرض الاصدار");
    println!("  عربي pakg <امر>        ادارة الحزم");
    println!();
    println!("\x1b[1;33mاوامر الحزم:\x1b[0m");
    println!("  عربي pakg create <اسم>   انشاء حزمة جديدة");
    println!("  عربي pakg list           عرض الحزم المثبتة");
    println!("  عربي pakg info <اسم>    عرض معلومات حزمة");
    println!("  عربي pakg remove <اسم>  حذف حزمة");
    println!("  عربي pakg update <اسم>  تحديث اصدار حزمة");
    println!();
    println!("\x1b[1;33mاوامر REPL:\x1b[0m");
    println!("  /مساعدة                عرض هذه المساعدة");
    println!("  /خروج                 الخروج");
    println!("  /حالة                 عرض المتغيرات");
    println!();
    println!("\x1b[1;33mامثلة:\x1b[0m");
    println!("  عربي program.عربي");
    println!("  عربي run program.عربي");
    println!("  عربي pakg create حزمة_جديدة");
    println!();
}

fn print_version() {
    println!("عَرَبِي v{}", VERSION);
}

fn pakg_create(name: &str) -> Result<()> {
    let pakgs_dir = std::path::PathBuf::from("__pakgs__");
    if !pakgs_dir.exists() {
        fs::create_dir_all(&pakgs_dir)?;
    }
    let pkg_dir = pakgs_dir.join(name);
    if pkg_dir.exists() {
        eprintln!("\x1b[1;31mخطا:\x1b[0m الحزمة '{}' موجودة بالفعل", name);
        std::process::exit(1);
    }
    fs::create_dir_all(&pkg_dir)?;
    let init_content = format!("// حزمة {}\n// اضف الكود هنا\n", name);
    fs::write(pkg_dir.join("__beginning__.txt"), &init_content)?;
    let manifest = format!(
        r#"{{"name":"{}","version":"0.1.0","description":"","author":"","files":["__beginning__.txt"]}}"#,
        name
    );
    fs::write(pkg_dir.join("pakg.json"), &manifest)?;
    println!("\x1b[1;32mتم انشاء الحزمة:\x1b[0m {}", name);
    println!("  __pakgs__/{}/", name);
    println!("  __beginning__.txt");
    println!("  pakg.json");
    Ok(())
}

fn pakg_list() -> Result<()> {
    let pakgs_dir = std::path::PathBuf::from("__pakgs__");
    if !pakgs_dir.exists() {
        println!("\x1b[2m(لا توجد حزم مثبتة)\x1b[0m");
        return Ok(());
    }
    let mut packages: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&pakgs_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                packages.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    packages.sort();
    if packages.is_empty() {
        println!("\x1b[2m(لا توجد حزم مثبتة)\x1b[0m");
    } else {
        println!("\x1b[1;36mالحزم المثبتة:\x1b[0m");
        for name in &packages {
            let manifest_path = pakgs_dir.join(name).join("pakg.json");
            let version = if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    json.get("version").and_then(|v| v.as_str()).unwrap_or("?").to_string()
                } else { "?".to_string() }
            } else { "?".to_string() };
            println!("  \x1b[1;32m{}\x1b[0m v{}", name, version);
        }
    }
    Ok(())
}

fn pakg_info(name: &str) -> Result<()> {
    let pakgs_dir = std::path::PathBuf::from("__pakgs__");
    let pkg_dir = pakgs_dir.join(name);
    if !pkg_dir.exists() {
        eprintln!("\x1b[1;31mخطا:\x1b[0m الحزمة '{}' غير موجودة", name);
        std::process::exit(1);
    }
    let manifest_path = pkg_dir.join("pakg.json");
    if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path)?;
        println!("\x1b[1;36mمعلومات الحزمة:\x1b[0m");
        println!("  {}", content);
    } else {
        println!("\x1b[1;33م الحزمة '{}' موجودة لكن ليس لها pakg.json\x1b[0m", name);
    }
    println!("\n\x1b[1;33مالملفات:\x1b[0m");
    if let Ok(entries) = fs::read_dir(&pkg_dir) {
        for entry in entries.flatten() {
            println!("  {}", entry.file_name().to_string_lossy());
        }
    }
    Ok(())
}

fn pakg_remove(name: &str) -> Result<()> {
    let pakgs_dir = std::path::PathBuf::from("__pakgs__");
    let pkg_dir = pakgs_dir.join(name);
    if !pkg_dir.exists() {
        eprintln!("\x1b[1;31mخطا:\x1b[0m الحزمة '{}' غير موجودة", name);
        std::process::exit(1);
    }
    fs::remove_dir_all(&pkg_dir)?;
    println!("\x1b[1;32mتم حذف الحزمة:\x1b[0m {}", name);
    Ok(())
}

fn pakg_update(name: &str) -> Result<()> {
    let pakgs_dir = std::path::PathBuf::from("__pakgs__");
    let pkg_dir = pakgs_dir.join(name);
    if !pkg_dir.exists() {
        eprintln!("\x1b[1;31mخطا:\x1b[0m الحزمة '{}' غير موجودة", name);
        std::process::exit(1);
    }
    let manifest_path = pkg_dir.join("pakg.json");
    if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path)?;
        if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(ver) = json.get("version").and_then(|v| v.as_str()) {
                let parts: Vec<&str> = ver.split('.').collect();
                if parts.len() == 3 {
                    if let Ok(patch) = parts[2].parse::<u32>() {
                        let new_ver = format!("{}.{}.{}", parts[0], parts[1], patch + 1);
                        json["version"] = serde_json::Value::String(new_ver.clone());
                        let updated = serde_json::to_string(&json)?;
                        fs::write(&manifest_path, &updated)?;
                        println!("\x1b[1;32mتم تحديث الحزمة:\x1b[0m {} → v{}", name, new_ver);
                        return Ok(());
                    }
                }
            }
            println!("\x1b[1;33م تم تغيير الاصدار — تاكد من تنسيق pakg.json\x1b[0m");
        } else {
            eprintln!("\x1b[1;31mخطا:\x1b[0m pakg.json غير صالح للـ JSON");
            std::process::exit(1);
        }
    } else {
        eprintln!("\x1b[1;31mخطا:\x1b[0m pakg.json غير موجود في الحزمة '{}'", name);
        std::process::exit(1);
    }
    Ok(())
}

fn pakg_help() {
    println!("\x1b[1;33marabi pakg — ادارة الحزم:\x1b[0m");
    println!("  pakg create <اسم>   انشاء حزمة جديدة");
    println!("  pakg list            عرض الحزم المثبتة");
    println!("  pakg info <اسم>     عرض معلومات حزمة");
    println!("  pakg remove <اسم>   حذف حزمة");
    println!("  pakg update <اسم>   تحديث اصدار حزمة");
}

fn print_vars(vm: &arabi_vm::VM) {
    let globals = &vm.globals;
    if globals.is_empty() {
        println!("\x1b[2m(لا توجد متغيرات)\x1b[0m");
        return;
    }
    let mut vars: Vec<_> = globals.iter().collect();
    vars.sort_by_key(|(k, _)| (*k).clone());
    for (name, value) in vars {
        println!("  \x1b[1;32m{}\x1b[0m = {}", name, value);
    }
}

fn repl() -> Result<()> {
    let mut rl = DefaultEditor::new().context("فشل في انشاء REPL")?;
    println!("\x1b[1;36m═══════════════════════════════════════\x1b[0m");
    println!("\x1b[1;36m       عَرَبِي — لغة البرمجة العربية\x1b[0m");
    println!("\x1b[1;36m          REPL التفاعلي v{}\x1b[0m", VERSION);
    println!("\x1b[1;36m═══════════════════════════════════════\x1b[0m");
    println!("\x1b[2mاكتب /مساعدة للمساعدة  |  Ctrl+D للخروج\x1b[0m");
    println!();
    let mut vm = arabi_vm::VM::new();
    if let Ok(cwd) = std::env::current_dir() {
        vm.set_search_dirs(vec![cwd]);
    }
    let prompt = "\x1b[1;32mعربي>\x1b[0m ";
    loop {
        match rl.readline(prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                if trimmed == "/خروج" || trimmed == "/exit" { println!("مع السلامة!"); break; }
                if trimmed == "/مساعدة" || trimmed == "/help" { print_help(); continue; }
                if trimmed == "/حالة" || trimmed == "/vars" { print_vars(&vm); continue; }
                let mut lexer = arabi_lexer::Lexer::new(trimmed);
                let tokens = match lexer.tokenize() { Ok(t) => t, Err(e) => { eprintln!("\x1b[1;31mخطا تحليل:\x1b[0m {}", e); continue; } };
                let mut parser = arabi_parser::Parser::new(tokens);
                let ast = match parser.parse() { Ok(a) => a, Err(e) => { eprintln!("\x1b[1;31mخطا تحليل:\x1b[0m {}", e); continue; } };
                let mut compiler = arabi_compiler::Compiler::new();
                let mut bytecode = match compiler.compile(&ast) { Ok(b) => b, Err(e) => { eprintln!("\x1b[1;31mخطا تجميع:\x1b[0m {}", e); continue; } };
                match vm.execute(&mut bytecode) { Ok(_) => {}, Err(e) => { eprintln!("\x1b[1;31mخطا تنفيذ:\x1b[0m {}", e); } }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => { println!("\x1b[2m(ايقاف)\x1b[0m"); }
            Err(rustyline::error::ReadlineError::Eof) => { println!("مع السلامة!"); break; }
            Err(e) => { eprintln!("\x1b[1;31mخطا:\x1b[0m {}", e); break; }
        }
    }
    Ok(())
}

fn run_file(filename: &str, debug: bool) -> Result<()> {
    let source = fs::read_to_string(filename).context(format!("فشل في قراءة الملف: {}", filename))?;
    let mut lexer = arabi_lexer::Lexer::new(&source);
    let tokens = lexer.tokenize().context("خطا في التحليل")?;
    let mut parser = arabi_parser::Parser::new(tokens);
    let ast = parser.parse().context("خطا في التوزيع")?;
    let mut compiler = arabi_compiler::Compiler::new();
    let mut bytecode = compiler.compile(&ast).context("خطا في التجميع")?;
    if debug {
        eprintln!("\x1b[1;35m═══ تفاصيل التجميع ═══\x1b[0m");
        for (i, instr) in bytecode.instructions.iter().enumerate() {
            eprintln!("  \x1b[2m{:4}:\x1b[0m {:?} (\x1b[33m{}\x1b[0m)", i, instr.opcode, instr.operand);
        }
        eprintln!("\x1b[1;35m═══════════════════════\x1b[0m");
    }
    let mut vm = arabi_vm::VM::new();
    let mut search_dirs = Vec::new();
    if let Ok(path) = std::fs::canonicalize(filename) {
        if let Some(parent) = path.parent() { search_dirs.push(parent.to_path_buf()); }
    }
    if let Ok(cwd) = std::env::current_dir() { search_dirs.push(cwd); }
    vm.set_search_dirs(search_dirs);
    vm.execute(&mut bytecode)?;
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => repl(),
        2 => match args[1].as_str() {
            "--help" | "-h" => { print_help(); Ok(()) }
            "--version" | "-v" => { print_version(); Ok(()) }
            "pakg" => { pakg_help(); std::process::exit(1); }
            _ => run_file(&args[1], false),
        },
        3 => if args[1] == "-d" || args[1] == "--debug" {
            run_file(&args[2], true)
        } else if args[1] == "pakg" {
            match args[2].as_str() {
                "list" => pakg_list(),
                _ => { eprintln!("امر غير معروف: {} — جرب: create, list, info, remove, update", args[2]); std::process::exit(1); }
            }
        } else if args[1] == "run" {
            run_file(&args[2], false)
        } else {
            eprintln!("معامل غير معروف: {}", args[1]); std::process::exit(1);
        },
        4 => if args[1] == "pakg" {
            match args[2].as_str() {
                "create" => pakg_create(&args[3]),
                "info" => pakg_info(&args[3]),
                "remove" => pakg_remove(&args[3]),
                "update" => pakg_update(&args[3]),
                _ => { eprintln!("امر غير معروف: {} — جرب: create, list, info, remove, update", args[2]); std::process::exit(1); }
            }
        } else if args[1] == "run" {
            if args[2] == "-d" || args[2] == "--debug" {
                if args.len() > 3 { run_file(&args[3], true) } else { eprintln!("يجب تحديد ملف"); std::process::exit(1); }
            } else {
                run_file(&args[3], false)
            }
        } else {
            eprintln!("معامل غير معروف: {}", args[1]); std::process::exit(1);
        },
        _ => {
            if args[1] == "run" && args.len() > 2 {
                let debug = args[2] == "-d" || args[2] == "--debug";
                if debug {
                    if args.len() > 3 { run_file(&args[3], true) } else { eprintln!("يجب تحديد ملف"); std::process::exit(1); }
                } else {
                    run_file(&args[2], false)
                }
            } else {
                eprintln!("عدد معاملات غير صحيح"); std::process::exit(1);
            }
        }
    }
}
