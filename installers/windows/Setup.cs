using System;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Windows.Forms;

[assembly: AssemblyTitle("DevLang Setup")]
[assembly: AssemblyDescription("Offline DevLang compiler, runtime, standard library and examples installer")]
[assembly: AssemblyCompany("d-osc")]
[assembly: AssemblyProduct("DevLang")]
[assembly: AssemblyVersion("@ASSEMBLY_VERSION@")]
[assembly: AssemblyFileVersion("@ASSEMBLY_VERSION@")]

internal static class Setup {
    internal const string Version = "@VERSION@";
    internal const string PayloadHash = "@PAYLOAD_HASH@";
    internal static string DefaultPrefix {
        get { return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "DevLang"); }
    }

    [STAThread]
    private static int Main(string[] args) {
        try {
            string prefix = DefaultPrefix;
            string log = null;
            bool silent = false, noPath = false, noRegistration = false;
            for (int i = 0; i < args.Length; i++) {
                switch (args[i]) {
                    case "--silent": silent = true; break;
                    case "--prefix": prefix = args[++i]; break;
                    case "--log": log = args[++i]; break;
                    case "--no-path": noPath = true; break;
                    case "--no-registration": noRegistration = true; break;
                    default: throw new ArgumentException("Unknown option: " + args[i]);
                }
            }
            if (silent) {
                string output;
                int status = Install(prefix, noPath, noRegistration, out output);
                if (log != null) File.WriteAllText(log, output, Encoding.UTF8);
                return status;
            }
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            Application.Run(new SetupWindow(prefix, noPath, noRegistration));
            return 0;
        } catch (Exception error) {
            // Silent mode never blocks automation on a dialog.
            if (Array.IndexOf(args, "--silent") < 0)
                MessageBox.Show(error.Message, "DevLang Setup", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }
    }

    private static void Extract(string resource, string destination) {
        using (Stream input = Assembly.GetExecutingAssembly().GetManifestResourceStream(resource))
        using (FileStream output = File.Create(destination)) {
            if (input == null) throw new InvalidDataException("Missing installer resource: " + resource);
            input.CopyTo(output);
        }
    }

    // Windows command-line escaping preserves spaces and trailing backslashes.
    private static string Quote(string value) {
        StringBuilder result = new StringBuilder("\"");
        int slashes = 0;
        foreach (char c in value) {
            if (c == '\\') { slashes++; continue; }
            if (c == '"') result.Append('\\', slashes * 2 + 1);
            else result.Append('\\', slashes);
            result.Append(c);
            slashes = 0;
        }
        result.Append('\\', slashes * 2).Append('"');
        return result.ToString();
    }

    internal static int Install(string prefix, bool noPath, bool noRegistration, out string output) {
        string stage = Path.Combine(Path.GetTempPath(), "devlang-setup-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(stage);
        try {
            string payload = Path.Combine(stage, "payload.zip");
            string script = Path.Combine(stage, "install.ps1");
            Extract("DevLang.Payload", payload);
            Extract("DevLang.InstallScript", script);
            using (SHA256 sha = SHA256.Create())
            using (FileStream stream = File.OpenRead(payload)) {
                string actual = BitConverter.ToString(sha.ComputeHash(stream)).Replace("-", "").ToLowerInvariant();
                if (actual != PayloadHash) throw new InvalidDataException("Installer payload checksum mismatch.");
            }
            ProcessStartInfo start = new ProcessStartInfo();
            start.FileName = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), "WindowsPowerShell", "v1.0", "powershell.exe");
            start.Arguments = "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File " + Quote(script) + " -Prefix " + Quote(Path.GetFullPath(prefix));
            if (noPath) start.Arguments += " -NoPath";
            if (noRegistration) start.Arguments += " -NoRegistration";
            start.UseShellExecute = false;
            start.CreateNoWindow = true;
            start.RedirectStandardOutput = true;
            start.RedirectStandardError = true;
            StringBuilder messages = new StringBuilder();
            using (Process process = new Process()) {
                process.StartInfo = start;
                process.OutputDataReceived += delegate(object sender, DataReceivedEventArgs e) { if (e.Data != null) lock (messages) messages.AppendLine(e.Data); };
                process.ErrorDataReceived += delegate(object sender, DataReceivedEventArgs e) { if (e.Data != null) lock (messages) messages.AppendLine(e.Data); };
                process.Start();
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();
                process.WaitForExit();
                output = messages.ToString();
                return process.ExitCode;
            }
        } finally {
            // The unique directory is owned entirely by this invocation.
            if (Path.GetFullPath(stage).StartsWith(Path.GetFullPath(Path.GetTempPath()), StringComparison.OrdinalIgnoreCase))
                Directory.Delete(stage, true);
        }
    }
}

internal sealed class SetupWindow : Form {
    private readonly TextBox destination = new TextBox();
    private readonly Button install = new Button();
    private readonly Button browse = new Button();
    private readonly TextBox status = new TextBox();
    private readonly bool noPath, noRegistration;
    private bool busy;

    internal SetupWindow(string prefix, bool skipPath, bool skipRegistration) {
        noPath = skipPath; noRegistration = skipRegistration;
        Text = "DevLang " + Setup.Version + " Setup";
        ClientSize = new Size(570, 355);
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        StartPosition = FormStartPosition.CenterScreen;
        Font = new Font("Segoe UI", 10);
        AutoScaleMode = AutoScaleMode.Dpi;
        Label heading = new Label { Text = "Install DevLang", Font = new Font("Segoe UI", 18, FontStyle.Bold), Location = new Point(24, 20), AutoSize = true };
        Label description = new Label { Text = "Compiler, runtime, standard library and examples.\nOffline installation for your account.", Location = new Point(26, 65), Size = new Size(520, 48) };
        Label location = new Label { Text = "Installation folder", Location = new Point(26, 119), AutoSize = true };
        destination.SetBounds(26, 145, 417, 27); destination.Text = prefix;
        browse.Text = "Browse..."; browse.SetBounds(451, 144, 92, 30);
        browse.Click += delegate { using (FolderBrowserDialog picker = new FolderBrowserDialog()) { picker.Description = "Choose the DevLang installation directory"; picker.SelectedPath = destination.Text; if (picker.ShowDialog(this) == DialogResult.OK) destination.Text = picker.SelectedPath; } };
        status.SetBounds(26, 190, 517, 96); status.Multiline = true; status.ReadOnly = true; status.ScrollBars = ScrollBars.Vertical;
        status.Text = "Click Install to set up DevLang and add d to your user PATH.\r\nNo administrator privileges or separate download required.";
        install.Text = "Install"; install.SetBounds(431, 305, 112, 32);
        install.Click += StartInstallation;
        Controls.AddRange(new Control[] { heading, description, location, destination, browse, status, install });
        AcceptButton = install;
        FormClosing += delegate(object sender, FormClosingEventArgs e) { if (busy) e.Cancel = true; };
    }

    private void StartInstallation(object sender, EventArgs e) {
        if (install.Text == "Close") { Close(); return; }
        string prefix = destination.Text;
        busy = true; install.Enabled = false; browse.Enabled = false; destination.Enabled = false;
        status.Text = "Verifying and installing DevLang...";
        ThreadPool.QueueUserWorkItem(delegate {
            string output;
            int code;
            try { code = Setup.Install(prefix, noPath, noRegistration, out output); }
            catch (Exception error) { code = 1; output = error.Message; }
            BeginInvoke((MethodInvoker)delegate {
                busy = false; install.Enabled = true;
                if (code == 0) { status.Text = "Installed successfully. Open a new terminal and run d --help.\r\nUninstall from Windows Installed apps.\r\n" + output; install.Text = "Close"; }
                else { status.Text = "Installation failed:\r\n" + output; browse.Enabled = true; destination.Enabled = true; }
            });
        });
    }
}
