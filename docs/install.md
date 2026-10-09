# Install DevLang

Offline installers include `d`, the compiler, source runtime, standard library and examples. They install for the current user without administrator privileges.

## Windows x86_64

Download `devlang-setup-v0.4.0-windows-x86_64.zip`, extract all files, and double-click `install.cmd`. Open a new terminal and run `d --help`. The default directory is `%LOCALAPPDATA%\Programs\DevLang`; the installer adds it to your user PATH and registers DevLang in Windows Installed apps.

Custom or unattended installation:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Prefix "C:\Tools\DevLang"
```

Use `-NoPath` to leave PATH unchanged, or `-NoRegistration` for a separate installation that does not register in Installed apps. Uninstall through Installed apps, or:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\DevLang\uninstall.ps1" -Uninstall
```

For a custom location, pass the same `-Prefix` when uninstalling. Restart the terminal after installation or removal.

## Linux x86_64

Requires Python 3.9+, glibc 2.39+ and libgcc. Download the `.py` installer and run:

```sh
python3 devlang-setup-v0.4.0-linux-x86_64.py
export PATH="$HOME/.local/bin:$PATH"
d --help
```

It installs into `~/.local/share/devlang` and links commands into `~/.local/bin`. Add that directory to your shell's PATH configuration if it is not already present. Shell startup files are not edited automatically.

Use `--prefix DIR --bin-dir DIR` for custom directories. To uninstall, rerun the same installer with `--uninstall` and the same directory options.

## Updates and native builds

Rerun the installer to update an existing managed installation. Files outside its recorded manifest are preserved, and conflicting unowned files/commands are rejected. Checksums are verified before installation. Verify the downloaded installer against the release `SHA256SUMS` as well.

Pure Dev source runs without a C compiler. Native builds and first-time C source FFI preparation require Clang/GCC on Windows. Linux includes TinyCC for basic fast builds; advanced C11 programs require Clang/GCC. Examples are in the installed `examples` directory; run them using their full path or from the installation directory. See the installed language and runtime documentation for stdlib module/link options.

These installers are not digitally signed.

Build release installers from the repository's prepared `dist` packages:

```sh
python scripts/build_installers.py --version v0.4.0
```
