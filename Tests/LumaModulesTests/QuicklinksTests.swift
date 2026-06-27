import Foundation
import Testing
@testable import LumaModules

@Test func quicklinkRendererEncodesOnlyVariableValues() {
    let rendered = QuicklinkTemplateRenderer.render(
        template: "https://github.com/search?q={{query}}&type=repositories",
        query: "swift package"
    )
    #expect(rendered == "https://github.com/search?q=swift%20package&type=repositories")
}

@Test func quicklinkRendererEncodesChineseAndEmojiQuery() {
    let rendered = QuicklinkTemplateRenderer.render(
        template: "https://www.google.com/search?q={{query}}",
        query: "中文 😀"
    )
    #expect(rendered.contains("%E4%B8%AD%E6%96%87%20%F0%9F%98%80"))
}

@Test func quicklinksIndexMatchesExactFirstTokenOnly() {
    let index = QuicklinksIndex(quicklinks: [
        Quicklink(name: "GitHub", trigger: "gh", urlTemplate: "https://github.com/search?q={{query}}")
    ])
    #expect(index.match(raw: "gh swift")?.query == "swift")
    #expect(index.match(raw: "github swift") == nil)
    #expect(index.match(raw: " gh  swift package ")?.query == "swift package")
}
