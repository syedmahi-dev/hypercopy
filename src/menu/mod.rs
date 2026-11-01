use colored::*;
use std::process::Command;

/// Register HyperCopy in the Windows File Explorer right-click context menu.
/// Uses HKCU (HKEY_CURRENT_USER), so it does NOT require Administrator privileges!
pub fn install_context_menu() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_path = current_exe.to_string_lossy();

    println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
    println!(" {}", "⚡ CONFIGURING WINDOWS EXPLORER CONTEXT MENU".bold().bright_yellow());
    println!(" Target binary: {}", exe_path.cyan());
    println!("{}", "──────────────────────────────────────────────────────────────────".bright_black());

    let reg_commands = [
        // 1. Right-click on any file -> "⚡ Copy with HyperCopy"
        ("HKCU\\Software\\Classes\\*\\shell\\HyperCopy", "", "⚡ Copy with HyperCopy"),
        ("HKCU\\Software\\Classes\\*\\shell\\HyperCopy", "Icon", "shell32.dll,134"),
        ("HKCU\\Software\\Classes\\*\\shell\\HyperCopy\\command", "", &format!("\"{}\" clip copy \"%1\"", exe_path)),

        // 2. Right-click on any folder -> "⚡ Copy with HyperCopy"
        ("HKCU\\Software\\Classes\\Directory\\shell\\HyperCopy", "", "⚡ Copy with HyperCopy"),
        ("HKCU\\Software\\Classes\\Directory\\shell\\HyperCopy", "Icon", "shell32.dll,134"),
        ("HKCU\\Software\\Classes\\Directory\\shell\\HyperCopy\\command", "", &format!("\"{}\" clip copy \"%1\"", exe_path)),

        // 3. Right-click inside any folder background -> "⚡ Paste with HyperCopy"
        ("HKCU\\Software\\Classes\\Directory\\Background\\shell\\HyperCopyPaste", "", "⚡ Paste with HyperCopy"),
        ("HKCU\\Software\\Classes\\Directory\\Background\\shell\\HyperCopyPaste", "Icon", "shell32.dll,260"),
        ("HKCU\\Software\\Classes\\Directory\\Background\\shell\\HyperCopyPaste\\command", "", &format!("cmd.exe /c \"\"{}\" paste \"%V\" & pause\"", exe_path)),

        // 4. Right-click on Desktop background -> "⚡ Paste with HyperCopy"
        ("HKCU\\Software\\Classes\\DesktopBackground\\shell\\HyperCopyPaste", "", "⚡ Paste with HyperCopy"),
        ("HKCU\\Software\\Classes\\DesktopBackground\\shell\\HyperCopyPaste", "Icon", "shell32.dll,260"),
        ("HKCU\\Software\\Classes\\DesktopBackground\\shell\\HyperCopyPaste\\command", "", &format!("cmd.exe /c \"\"{}\" paste \"%V\" & pause\"", exe_path)),
    ];

    for (key, val_name, val_data) in reg_commands {
        let mut cmd = Command::new("reg");
        cmd.arg("add").arg(key).arg("/f");
        if val_name.is_empty() {
            cmd.arg("/ve");
        } else {
            cmd.arg("/v").arg(val_name);
        }
        cmd.arg("/d").arg(val_data);

        let output = cmd.output().map_err(|e| format!("Failed to execute reg.exe: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("reg.exe error adding {}: {}", key, stderr));
        }
    }

    println!(" {} Windows File Explorer Right-Click Menu successfully installed!", "✅".bold().green());
    println!("   • Right-click any file or folder -> {}", "⚡ Copy with HyperCopy".bold().cyan());
    println!("   • Right-click empty folder background -> {}", "⚡ Paste with HyperCopy".bold().green());
    println!("{}", "══════════════════════════════════════════════════════════════════".bright_cyan());
    Ok(())
}

/// Remove HyperCopy from the Windows File Explorer context menu.
pub fn remove_context_menu() -> Result<(), String> {
    let keys = [
        "HKCU\\Software\\Classes\\*\\shell\\HyperCopy",
        "HKCU\\Software\\Classes\\Directory\\shell\\HyperCopy",
        "HKCU\\Software\\Classes\\Directory\\Background\\shell\\HyperCopyPaste",
        "HKCU\\Software\\Classes\\DesktopBackground\\shell\\HyperCopyPaste",
    ];

    for key in keys {
        let _ = Command::new("reg")
            .arg("delete")
            .arg(key)
            .arg("/f")
            .output();
    }

    println!("{} Windows Explorer Context Menu entries removed.", "✅".bold().green());
    Ok(())
}
