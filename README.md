# 錆の核 sabinokaku

Minimal library to inject .NET CoreCLR into a process via DLL-main. 
Windows only for now, Linux and macOS requires a slightly different approach with `LD_PRELOAD` that is not yet supported.

## Usage
1. Create a class library project for use as your entry point, and add `<GenerateRuntimeConfigurationFiles>True</GenerateRuntimeConfigurationFiles>` to the csproj to properly generate the runtime configuration. 
2. The entry point in .NET must always have the signature `public static int Main(IntPtr args, int sizeBytes)`. 
3. Create a `kaku.co` file, see [Configuration](#configuration) for syntax, and add it to your project.
4. Add a prebuilt binary of `sabinokaku_win.dll`

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
</ItemGroup>
```

5. Use the tool of your choice to inject `sabinokaku_win.dll`, which will automatically bootstrap the CLR, and call your entry point function. 
 
## Configuration

To determine the .NET bootstrap point, sabinokaku requires a `kaku.co` file either in the same directory as `sabinokaku_win.dll`, or in the host process directory.
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
