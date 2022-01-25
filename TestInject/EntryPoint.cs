using System.Runtime.InteropServices;
namespace TestInject;
public static class EntryPoint
{
    [DllImport("kernel32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    static extern bool AllocConsole();

        public static int Main(IntPtr args, int sizeBytes) {
            AllocConsole();
            Console.WriteLine("Hello from C#!");
            Console.ReadLine();
            return 42;
        }
}
