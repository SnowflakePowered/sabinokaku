# 錆の核 sabinokaku

Minimal framework to inject the .NET Runtime into a process.

Supports Windows and Linux. macOS support is possible via `LD_PRELOAD` but will require slightly different hooks than 
included for Linux.

## Usage
1. Create a class library project for use as your entry point, and add `<GenerateRuntimeConfigurationFiles>True</GenerateRuntimeConfigurationFiles>` to the csproj to properly generate the runtime configuration. 
2. The entry point in .NET must always have the signature `public static int Main(IntPtr args, int sizeBytes)`. 
3. Create a `kaku.co` file, see [Configuration](#configuration) for syntax, and add it to your project.
4. Add a prebuilt binary of `kaku.dll` on Windows, or `libkaku.so` on Linux.

Your csproj should have the following entries.

```xml
<PropertyGroup>
  <GenerateRuntimeConfigurationFiles>True</GenerateRuntimeConfigurationFiles>
</PropertyGroup>
<ItemGroup>
  <None Update="kaku.co">
    <CopyToOutputDirectory>Always</CopyToOutputDirectory>
  </None>
  <None Update="kaku.dll">
    <CopyToOutputDirectory>Always</CopyToOutputDirectory>
  </None>
  
  <!-- Linux only -->
  <None Update="libkaku.so">
      <CopyToOutputDirectory>Always</CopyToOutputDirectory>
  </None>
</ItemGroup>
```

5. On Windows, inject `kaku.dll` with a DLL injection tool such as [Reloaded.Injector](https://github.com/Reloaded-Project/Reloaded.Injector). On Linux, use `LD_PRELOAD` to inject `libkaku.so`.
   On load, the CLR will be bootstrapped on a separate thread and your entry point function will be called.

Note that the lifetime of the host process *always* outlives the lifetime of the .NET Runtime thread. If the host process
does not live long enough for the .NET Runtime to bootstrap and finish execution, it will be killed along with the host process. 

Waiting for the thread to join before the process quits is doable, but not implemented since this leads to some unexpected behaviour such as
zombie processes. If there seems to be such a need, please file an issue; I will consider making it configurable via `kaku.co`.
 
## Configuration

To determine the .NET bootstrap point, sabinokaku requires a `kaku.co` file either in the same directory as `kaku.dll`/`libkaku.so`, or in the host process directory.
There are 2 formats that sabinokaku understands. The long format (`kaku_l`) allows for the most flexibility if you have weird AssemblyConfig options, but the short format is generally preferred. 

### Long Format Preamble
The long format always begins with `kaku_l`, followed by the relative path to `runtimeconfig.json`, the relative path to the .NET assembly DLL, the qualified name of the entry-point class, and the name of the entry-point function.

```
kaku_l
TestInject.runtimeconfig.json
TestInject.dll
TestInject.EntryPoint, TestInject
Main
```

### Short Format Preamble
The short format always begins with `kaku_s`. Information is then inferred with the syntax `AssemblyName::QualifiedClassName$EntryFunction`

```
kaku_s
TestInject::TestInject.EntryPoint$Main
```

### Setting Environment Variables
After the long or short format preamble, you may provide **optional** environment variables to be set before hostfxr is invoked and the .NET runtime is bootstrapped.
The format is `env KEY=VAR`, be sure there are no spaces between the equals symbol or it will be taken as part of the environment string.

For example, to set `DOTNET_MULTILEVEL_LOOKUP=0`, an example `kaku.co` may be
```
kaku_s
TestInject::TestInject.EntryPoint$Main
env DOTNET_MULTILEVEL_LOOKUP=0
```

Multiple environment variables can be set.
```
kaku_s
TestInject::TestInject.EntryPoint$Main
env DOTNET_MULTILEVEL_LOOKUP=0
env COMPLUS_ForceENC=1
```
Note that any set variables will also take effect against the hosting process after initialization. sabinokaku will not restore the prior values, so use this feature
with caution.
