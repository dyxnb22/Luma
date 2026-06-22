import LumaCore

public enum BuiltInModules {
    public static func makeAll() -> [any LumaModule] {
        [
            AppsModule(),
            WindowsModule(),
            ClipboardModule(),
            CommandsModule(),
            CalculatorModule(),
            TranslateModule()
        ]
    }
}
