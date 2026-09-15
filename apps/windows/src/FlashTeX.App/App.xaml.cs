// name: App.xaml.cs
// purpose: WinUI3 application entry point. Feasibility spike for building a
//   WinUI3 head via plain `dotnet build` (NuGet WindowsAppSDK packages), not
//   the Visual Studio "Windows application development" workload, which is
//   not installed on this machine.
// author: Claude Sonnet 5
// date: 2026-09-14

using Microsoft.UI.Xaml;

namespace FlashTeX.App;

public partial class App : Application
{
    private Window? _window;

    public App()
    {
        InitializeComponent();
    }

    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _window = new MainWindow();
        _window.Activate();
    }
}
