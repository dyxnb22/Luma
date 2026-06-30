import Foundation
import LumaCore
import LumaModules

struct HomeContinueClientAdapters {
    struct Notes: NotesContinueClient {
        let module: NotesModule

        func dailyNotePath() async -> String? {
            await module.dailyNotePath()
        }
    }

    struct Todo: TodoContinueClient {
        let module: TodoModule

        func firstTodayDueReminder() async throws -> ReminderSnapshot? {
            try await module.firstTodayDueReminder()
        }
    }

    struct Media: MediaContinueClient {
        let module: MediaModule

        func inProgressCount() async -> Int {
            await module.inProgressCount()
        }
    }

    struct Wordbook: WordbookContinueClient {
        let module: WordbookModule

        func dueTodayCount() async -> Int {
            await module.storeDueTodayCount()
        }
    }
}
