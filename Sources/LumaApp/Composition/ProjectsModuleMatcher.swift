import Foundation
import LumaCore
import LumaModules

struct ProjectsModuleMatcher: ProjectMatcherClient {
    let module: ProjectsModule

    func match(label: String) async -> MatchedProject? {
        guard let record = await module.matchByLabel(label) else { return nil }
        return MatchedProject(path: record.path, name: record.name)
    }
}
