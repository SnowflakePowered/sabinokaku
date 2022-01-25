# 錆の核 sabinokaku

Minimal framework to inject the .NET Runtime into a process.

Supports Windows and Linux. macOS support is possible via `LD_PRELOAD` but will require slightly different hooks than 
included for Linux.

## Usage
1. Create a class library project for use as your entry point, and add `<GenerateRuntimeConfigurationFiles>True</GenerateRuntimeConfigurationFiles>` to the csproj to properly generate the runtime configuration. 
2. The entry point in .NET must always have the signature `public static int Main(IntPtr args, int sizeBytes)`. 
3. Create a `kaku.co` file, see [Configuration](#configuration) for syntax, and add it to your project.
4. Add a prebuilt binary of `sabinokaku_win.dll`, or `sabinokaku_linux.so` on Linux.

Your csproj should have the following entries.

```xml
<PropertyGroup>
  <GenerateRuntimeConfigurationFiles>True</GenerateRuntimeConfigurationFiles>
</PropertyGroup>
<ItemGroup>
  <None Update="kaku.co">
    <CopyToOutputDirectory>Always</CopyToOutputDirectory>
  </None>
  <None Update="sabinokaku_win.dll">
    <CopyToOutputDirectory>Always</CopyToOutputDirectory>
  </None>
  
  <!-- Linux only -->
  <None Update="sabinokaku_linux.so">
      <CopyToOutputDirectory>Always</CopyToOutputDirectory>
  </None>
</ItemGroup>
```

5. On Windows, inject `sabinokaku_win.dll` with `LoadLibraryW`. On Linux, use `LD_PRELOAD` to inject `sabinokaku_linux.so`.
   On load, the CLR will be bootstrapped on a separate thread and your entry point function will be called.

Note that the lifetime of the host process *always* outlives the lifetime of the .NET Runtime thread. If the host process
does not live long enough for the .NET Runtime to bootstrap and finish execution, it will be killed along with the host process. 

Waiting for the thread to join before the process quits is doable, but not implemented since this leads to some unexpected behaviour such as
zombie processes. If there seems to be such a need, please file an issue; I will consider making it configurable via `kaku.co`.
 
## Configuration

To determine the .NET bootstrap point, sabinokaku requires a `kaku.co` file either in the same directory as `sabinokaku_win.dll`/`sabinokaku_linux.so`, or in the host process directory.
There are 2 formats that sabinokaku understands. The long format (`kaku_l`) allows for the most flexibility if you have weird AssemblyConfig options, but the short format is generally preferred. 

### Long format example
The long format always begins with `kaku_l`, followed by the relative path to `runtimeconfig.json`, the relative path to the .NET assembly DLL, the qualified name of the entry-point class, and the name of the entry-point function.

```
kaku_l
TestInject.runtimeconfig.json
TestInject.dll
TestInject.EntryPoint, TestInject
Main
```

### Short format example
The short format always begins with `kaku_s`. Information is then inferred with the syntax `AssemblyName::QualifiedClassName$EntryFunction`

```
kaku_s
TestInject::TestInject.EntryPoint$Main
```
