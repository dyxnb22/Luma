import Foundation

/// Home provider that splits contextual suggestions into continue vs create sections.
public protocol ContextualHomeSectionProvider: LauncherHomeProvider {
    func sectionedItems() async -> (continue: [ResultItem], create: [ResultItem])
}

extension ContextualHomeSectionProvider {
    public func sectionedItems() async -> (continue: [ResultItem], create: [ResultItem]) {
        let items = await self.items()
        return (continue: [], create: items)
    }
}
