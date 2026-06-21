import Testing
@testable import LumaModules

@Test func builtInModulesAreRegistered() {
    #expect(BuiltInModules.makeAll().isEmpty == false)
}
