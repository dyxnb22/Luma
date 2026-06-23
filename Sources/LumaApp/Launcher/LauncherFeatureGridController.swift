import AppKit
import LumaCore
import LumaInfrastructure
import LumaModules

@MainActor
final class LauncherFeatureGridController {
    private let featureGridView: FeatureFlowView
    private let sortedCards: [FeatureCard]
    private let onCardTapped: (FeatureCard) -> Void

    private(set) var widgetCards: [WidgetFeatureCard] = []
    private var clipboardStatusSummary = "Loading…"
    private var wordbookDueCount = 0
    private var todoDueCount = 0
    private var subscriptionTask: Task<Void, Never>?

    init(
        featureGridView: FeatureFlowView,
        sortedCards: [FeatureCard],
        onCardTapped: @escaping (FeatureCard) -> Void
    ) {
        self.featureGridView = featureGridView
        self.sortedCards = sortedCards
        self.onCardTapped = onCardTapped
    }

    func startSubscriptions() {
        subscriptionTask?.cancel()
        subscriptionTask = Task { [weak self] in
            await withTaskGroup(of: Void.self) { group in
                group.addTask {
                    for await _ in WordbookStoreChangeHub.dataChanges() {
                        await MainActor.run { self?.refreshStatuses() }
                    }
                }
                group.addTask {
                    for await _ in ClipboardStoreChangeHub.dataChanges() {
                        await MainActor.run { self?.refreshStatuses() }
                    }
                }
                group.addTask {
                    for await _ in TodoChangeHub.dataChanges() {
                        await MainActor.run { self?.refreshStatuses() }
                    }
                }
            }
        }
        refreshStatuses()
    }

    func stopSubscriptions() {
        subscriptionTask?.cancel()
        subscriptionTask = nil
    }

    func render() {
        featureGridView.subviews.forEach { $0.removeFromSuperview() }
        widgetCards = []
        for (index, card) in sortedCards.enumerated() {
            let widget = WidgetFeatureCard(
                card: card,
                shortcutIndex: index + 1,
                statusSummary: statusSummary(for: card)
            ) { [weak self] selected in
                self?.onCardTapped(selected)
            }
            widget.translatesAutoresizingMaskIntoConstraints = true
            widgetCards.append(widget)
            featureGridView.addSubview(widget)
            applyBadge(for: card, to: widget)
        }
        featureGridView.cardViews = widgetCards
    }

    func updateHighlight(index: Int?) {
        for (cardIndex, widget) in widgetCards.enumerated() {
            widget.setHighlighted(cardIndex == index)
        }
    }

    func refreshStatuses() {
        Task { [weak self] in
            guard let self else { return }
            let clipStats: ClipboardStatistics
            if let module = ModuleDetailRegistry.clipboardModule {
                clipStats = await module.statistics()
            } else {
                clipStats = ClipboardStatistics(total: 0, pinned: 0)
            }
            let wordbookDue: Int
            if let store = ModuleDetailRegistry.wordbookStore {
                wordbookDue = (try? await store.dueTodayCount()) ?? 0
            } else {
                wordbookDue = 0
            }
            let todoDue: Int
            if let module = ModuleDetailRegistry.todoModule {
                todoDue = (try? await module.todayDueCount()) ?? 0
            } else {
                todoDue = 0
            }
            let targetLang = await ModuleDetailRegistry.config?.translationTargetLanguage() ?? "en"
            await MainActor.run {
                self.clipboardStatusSummary = "\(clipStats.total) entries · \(clipStats.pinned) pinned"
                self.wordbookDueCount = wordbookDue
                self.todoDueCount = todoDue
                TranslateDashboardStatus.targetLanguageCode = targetLang
                for (index, card) in self.sortedCards.enumerated() where self.widgetCards.indices.contains(index) {
                    self.widgetCards[index].updateStatusSummary(self.statusSummary(for: card))
                    self.applyBadge(for: card, to: self.widgetCards[index])
                }
            }
        }
    }

    private func statusSummary(for card: FeatureCard) -> String {
        switch card.id {
        case .translate:
            let lang = languageDisplayName(for: TranslateDashboardStatus.targetLanguageCode)
            return "→ \(lang) · \(TranslateDashboardStatus.summary)"
        case .clipboard:
            return clipboardStatusSummary
        case .wordbook:
            return wordbookDueCount > 0 ? "\(wordbookDueCount) due today" : "Vocabulary review"
        case .todo:
            return todoDueCount > 0 ? "\(todoDueCount) due today" : "Reminders capture"
        default:
            // Return empty so statusLabel is hidden — subtitle already shows card.subtitle above it.
            return ""
        }
    }

    private func applyBadge(for card: FeatureCard, to widget: WidgetFeatureCard) {
        switch card.id {
        case .wordbook:
            widget.setBadgeCount(wordbookDueCount > 0 ? wordbookDueCount : nil)
        case .todo:
            widget.setBadgeCount(todoDueCount > 0 ? todoDueCount : nil)
        default:
            widget.setBadgeCount(nil)
        }
    }

    private func languageDisplayName(for code: String) -> String {
        switch code {
        case "en": return "English"
        case "zh-Hans": return "Chinese (Simplified)"
        case "zh-Hant": return "Chinese (Traditional)"
        case "ja": return "Japanese"
        case "ko": return "Korean"
        case "es": return "Spanish"
        case "fr": return "French"
        case "de": return "German"
        default: return code
        }
    }
}
