# 錆の核 sabinokaku

Minimal framework to inject the .NET Runtime into a process.

Supports Windows and Linux. macOS support is complicated due to SIP, and will not be covered here.

## Building
Building sabinokaku requires the nightly toolchain. The workspace Cargo.toml is preconfigured to strip debug symbols, 
otherwise the Linux `kaku.so` binary will bloat from ~500KB to around 3MB large. 

```
$ cargo build --release
```

The `sabinokaku-common` module contains platform independent logic such as configuration parsing and bootstrapping `hostfxr` and `nethost`. 
The `sabinokaku-loader` module contains the code for the actual injected assembly; this will be built as `cdylib` (`.dll` or `.so`), and is the
actual assembly that has to be injected into the address space of the target process.

`injector-example` is a simple dummy program that injects a DLL into a running process, or simply runs `Hello World` and 
dumps environment variables for testing injection on Linux.

## Usage
1. Create a class library project for use as your entry point, and add `<EnableDynamicLoading>True</EnableDynamicLoading>` to the csproj to properly generate the runtime configuration. 
2. The entry point in .NET must always have the signature `public static int Main(IntPtr args, int sizeBytes)`. 
3. Create a `kaku.co` file, see [Configuration](#configuration) for syntax, and add it to your project.
4. Add a prebuilt binary of `kaku.dll` on Windows, or `libkaku.so` on Linux.

Your csproj should have the following entries.

```xml
<PropertyGroup>
  <EnableDynamicLoading>true</EnableDynamicLoading>
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

5. On Windows, inject `kaku.dll` into a running process with a DLL injection tool such as [Reloaded.Injector](https://github.com/Reloaded-Project/Reloaded.Injector). 
   On Linux, `libkaku.so` hooks `__libc_start_main` and can be injected with `LD_PRELOAD`.
   On load, the CLR will be bootstrapped on a separate thread and your entry point function will be called.


Note that the lifetime of the host process *always* outlives the lifetime of the .NET Runtime thread. If the host process
does not live long enough for the .NET Runtime to bootstrap and finish execution, it will be killed along with the host process. 

Waiting for the thread to join before the process quits is doable, but not implemented since this leads to some unexpected behaviour such as
zombie processes. If there seems to be such a need, please file an issue; I will consider making it configurable via `kaku.co`.
 
## Configuration

To determine the .NET bootstrap point, sabinokaku requires a `kaku.co` file either in the same directory as `kaku.dll`/`libkaku.so`, 
or in the host process directory. `kaku.co` contains the preamble necessary for sabinokaku to bootstrap the .NET runtime. There are 2
preamble formats that sabinokaku understands. The long format (`kaku_l`) allows for the most flexibility, for example if you
store the .NET entry point assembly in a child folder. The short form may be preferred for its shorter syntax.

### Long Format Preamble
The long format always begins with `kaku_l`, followed by the path to `runtimeconfig.json`, the path to the .NET assembly DLL, 
the qualified name of the entry-point class, and the name of the entry-point function. All paths are relative to the location of
`kaku.co`.

```
kaku_l
TestInject.runtimeconfig.json
TestInject.dll
TestInject.EntryPoint, TestInject
Main
```

### Short Format Preamble
The short format always begins with `kaku_s`. Information is then inferred with the syntax `AssemblyName::QualifiedClassName$EntryFunction`. The
short format preamble requires that your assembly and runtime configuration file is in the same folder as `kaku.co`.

```
kaku_s
TestInject::TestInject.EntryPoint!Main
```
### Advanced Configuration

#### Setting Environment Variables
After the preamble, you may provide **optional** environment variables to be set before hostfxr is invoked and the .NET runtime is bootstrapped.
The format is `env KEY=VAR`, be sure there are no spaces between the equals symbol or it will be taken as part of the environment string.

For example, to set `DOTNET_MULTILEVEL_LOOKUP=0`, an example `kaku.co` may be
```
kaku_s
TestInject::TestInject.EntryPoint!Main
env DOTNET_MULTILEVEL_LOOKUP=0
```

Multiple environment variables can be set.
```
kaku_s
TestInject::TestInject.EntryPoint!Main
env DOTNET_MULTILEVEL_LOOKUP=0
env COMPLUS_ForceENC=1
```

The order of environment variables set is **not guaranteed**.

⚠️**Warning**⚠️

Any set variables will also take effect against the hosting process after initialization, and sabinokaku will not restore the prior values. 
See [Platform Differences](#platform-differences) for more details. 

#### Providing your own `hostfxr.dll`
After the preamble, you may **optionally** provide the path to your own `hostfxr.dll`, which is resolved
relative to `kaku.co`.

```
kaku_s
TestInject::TestInject.EntryPoint!Main
hostfxr runtime/host/hostfxr.dll
env DOTNET_MULTILEVEL_LOOKUP=0
```

sabinokaku will then try to load using your custom `hostfxr.dll`. If it does not exist, the runtime will fail to bootstrap.
If you specify `hostfxr` multiple times, only the first entry is taken.

#### Specifying a runtime
After the preamble, you may **optionally** provide the path to a dotnet root folder containing a `dotnet.exe` and a .NET
runtime, relative to `kaku.co`. Be sure to set `DOTNET_MULTILEVEL_LOOKUP=0` as well.

```
kaku_s
TestInject::TestInject.EntryPoint!Main
dotnetroot runtime
env DOTNET_MULTILEVEL_LOOKUP=0
```

sabinokaku will then try to load the runtime specified.

If you specify `dotnetroot` multiple times, only the first entry is taken.

#### Vulkan Hooking

sabinokaku provides specialized functionality for initializing the CLR for Vulkan hooking purposes as a layer. Your Vulkan
driver must support layer interface 2 (Vulkan 1.1). 

To load the CLR during Vulkan instantiation, you must enable it by setting the `vulkan` option in your `kaku.co`. You 
must specify a loader layer library version (must be greater than 2, or the layer will not load), and an entry point for the
runtime, either `CreateDevice` (called at first `vkCreateDevice`), or `CreateInstance` (called at first `vkCreateInstance`).

You must also set the environment variable `ENABLE_SABINOKAKU_VULKAN=1`, this will also enable the layer with the included `layer.json`
manifest. This will disable injection via `DllMain` or `_libc_start_main`.

```
kaku_s
TestInject::TestInject.EntryPoint!Main
vulkan 2 CreateDevice
env SABINOKAKU_VULKAN_BOOTED=1
```

You may then configure `kaku.dll` or `libkaku.so` as a Vulkan layer. See [the Vulkan documentation](https://vulkan.lunarg.com/doc/view/1.3.204.0/windows/loader_and_layer_interface.html#user-content-layer-manifest-file-format)
for more information.

On the first load of the layer, sabinokaku will pass a Vulkan handle of the initialized `VkInstance` or `VkDevice` as the arguments
to the .NET entry point. `VkInstance*` will **always** be the first handle passed. If `CreateDevice` is the entrypoint, `VkDevice*` will
be the second pointer passed. The memory ownership of the handles passed will become the CLRs, but since the allocator
is unknown in the managed context, it should be considered leaked memory.

sabinokaku will initialize the CLR **only on the first** calls to the layer function. To hook subsequent calls to `vkCreateInstance` or
`vkCreateDevice`, you must do so manually in managed code and hook the calls at the loader level. The returned pointers to `VkInstance` and
`VkDevice` can be used for hooking the instance or device call chain but **must** be updated if the Vulkan instance or device is recreated. If
an application creates multiple instances or devices in a short timeframe before managed code can hook, you will not be able to hook into
the subsequent call chains.

Because the Vulkan loader will **reinitialize all layers** on device recreation, you must also include `env SABINOKAKU_VULKAN_BOOTED=1` in your `kaku.co`
to prevent the layer from being reinitialized (and thus CLR) on device recreation.

## Platform Differences
Particularly when using the environment variables feature, note the differences in load order between Windows and Linux.

Windows uses `DllMain` and thus the host process is already executing when the runtime is injected. Changed environment 
variables will thus be visible to the host process only if they are read **after** initialization.

Linux hooks `__libc_start_main` and bootstraps the .NET runtime **before** `main`, thus any environment variables set **may** be visible
to the target process `main`.

On both platforms, .NET initialization and execution happens on a separate thread. However, if an uncaught exception occurs 
it is not allowed to cross the FFI boundary. To avoid undefined behaviour, the host application will be aborted.
