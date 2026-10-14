using System.Globalization;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

internal static class Program
{
    [DllImport("devbridge", CallingConvention = CallingConvention.Cdecl)]
    private static extern int bridge_add(int a, int b);
    [DllImport("devbridge", CallingConvention = CallingConvention.Cdecl)]
    private static extern double bridge_scale(double value, double factor);
    [DllImport("devbridge", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.I1)]
    private static extern bool bridge_positive(int value);
    [DllImport("devbridge", CallingConvention = CallingConvention.Cdecl)]
    private static extern IntPtr bridge_name();

    private static int Main(string[] args)
    {
        Console.InputEncoding = new UTF8Encoding(false);
        Console.OutputEncoding = new UTF8Encoding(false);
        try
        {
            if (args.Length == 1 && args[0] == "worker")
            {
                using var document = JsonDocument.Parse(Console.In.ReadToEnd());
                var request = document.RootElement;
                int a = request.GetProperty("a").GetInt32();
                int b = request.GetProperty("b").GetInt32();
                string name = request.GetProperty("name").GetString() ?? throw new Exception("name required");
                if (a < -1000000 || a > 1000000 || b < -1000000 || b > 1000000)
                    throw new Exception("a/b outside example range");
                Console.WriteLine(JsonSerializer.Serialize(new { sum = a + b, message = "Hello " + name }));
                return 0;
            }
            if (args.Length != 1) throw new Exception("Pass the shared library path, or worker");
            NativeLibrary.SetDllImportResolver(typeof(Program).Assembly,
                (name, _, _) => name == "devbridge" ? NativeLibrary.Load(Path.GetFullPath(args[0])) : IntPtr.Zero);
            Console.WriteLine(bridge_add(20, 22));
            Console.WriteLine(bridge_scale(2.5, 5.0).ToString(CultureInfo.InvariantCulture));
            Console.WriteLine(bridge_positive(7) ? "true" : "false");
            Console.WriteLine(bridge_positive(-7) ? "true" : "false");
            Console.WriteLine(Marshal.PtrToStringUTF8(bridge_name()));
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error.Message);
            return 2;
        }
    }
}
