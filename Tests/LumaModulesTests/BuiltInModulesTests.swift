import Testing
@testable import LumaModules

@Test func builtInModulesAreRegistered() {
    #expect(BuiltInModules.makeAll().isEmpty == false)
}

@Test func activeBuiltInModulesExcludeDeferredCalculatorAndWindows() {
    let ids = Set(BuiltInModules.makeAll().map { type(of: $0).manifest.identifier })
    #expect(!ids.contains(.calculator))
    #expect(!ids.contains(.windows))
    #expect(ids.contains(.translate))
    #expect(ids.contains(.clipboard))
}

@Test func deferredModulesRemainAvailableInSource() {
    let deferred = BuiltInModules.makeDeferred().map { type(of: $0).manifest.identifier }
    #expect(deferred.contains(.calculator))
    #expect(deferred.contains(.windows))
}

@Test func activeModulesDoNotRequireAccessibility() {
    #expect(BuiltInModules.activeModulesRequireAccessibility() == false)
}
