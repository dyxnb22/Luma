import Foundation
import LumaCore

public enum TemplateExpander {
    public static func expand(_ text: String, context: SnippetExpansionContext) -> String {
        SnippetVariableExpander.expand(text, context: context)
    }
}
