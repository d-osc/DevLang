# Install DevLang

Offline installers include `d`, the compiler, source runtime, standard library and examples. They install for the current user without administrator privileges.

## Windows x86_64

Download `devlang-setup-v0.5.3-windows-x86_64.exe`, double-click it, and click **Install**. Everything is embedded in this single file; no extraction or additional downloads are needed. Open a new terminal and run `d --help`. The default directory is `%LOCALAPPDATA%\Programs\DevLang`; the installer adds it to your user PATH and registers DevLang in Windows Installed apps. Windows 10/11 x86_64 with the built-in .NET Framework and Windows PowerShell is supported.

The ZIP installer remains available: extract all files and open `install.cmd`.

Custom or unattended installation:

```powershell
.\devlang-setup-v0.5.3-windows-x86_64.exe --silent --prefix "C:\Tools\DevLang" --log "install.log"
```

The EXE also accepts `--no-path` and `--no-registration`. For the ZIP installer:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Prefix "C:\Tools\DevLang"
```

Use `-NoPath` to leave PATH unchanged, or `-NoRegistration` for a separate installation that does not register in Installed apps. Uninstall through Installed apps, or:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\DevLang\uninstall.ps1" -Uninstall
```

For a custom location, pass the same `-Prefix` when uninstalling. Restart the terminal after installation or removal.

## Windows ARM64

Download `devlang-setup-v0.5.3-windows-arm64.zip`, extract it and run `install.cmd` on Windows 11 ARM64. The included d/devc/devrun binaries are native ARM64. The single-file GUI EXE is currently provided only for x86_64.

## Linux x86_64 and ARM64

Requires Python 3.9+, glibc 2.39+ and libgcc. Download the `.py` installer and run:

```sh
python3 devlang-setup-v0.5.3-linux-x86_64.py
# On ARM64 instead use devlang-setup-v0.5.3-linux-arm64.py
export PATH="$HOME/.local/bin:$PATH"
d --help
```

It installs into `~/.local/share/devlang` and links commands into `~/.local/bin`. Add that directory to your shell's PATH configuration if it is not already present. Shell startup files are not edited automatically.

Use `--prefix DIR --bin-dir DIR` for custom directories. To uninstall, rerun the same installer with `--uninstall` and the same directory options.

## Updates and native builds

Rerun the installer to update an existing managed installation. Files outside its recorded manifest are preserved, and conflicting unowned files/commands are rejected. Checksums are verified before installation. Verify the downloaded installer against the release `SHA256SUMS` as well.

Pure Dev source runs without a C compiler. Native builds and first-time C source FFI preparation require an installed Clang/GCC backend on each platform. This release does not bundle TinyCC. Examples are in the installed `examples` directory; run them using their full path or from the installation directory. See the installed language and runtime documentation for stdlib module/link options.

These installers are not digitally signed.

Build release installers from the repository's prepared `dist` packages:

```sh
python scripts/build_installers.py --version v0.5.3
python scripts/build_installers.py --version v0.5.3 --windows-exe
```
