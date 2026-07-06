import Foundation
import LumaCore
import Testing

@Test func notesDetailRefreshGateInvalidatesStaleGeneration() {
    var gate = NotesDetailRefreshGate()
    let first = gate.beginRefresh()
    gate.invalidate()
    #expect(gate.isCurrent(first) == false)
}

@Test func notesDetailRefreshGateSecondActivateInvalidatesFirst() {
    var gate = NotesDetailRefreshGate()
    let first = gate.beginRefresh()
    let second = gate.beginRefresh()
    #expect(gate.isCurrent(first) == false)
    #expect(gate.isCurrent(second) == true)
}

@Test func notesDetailRefreshGateCurrentTracksLatestBegin() {
    var gate = NotesDetailRefreshGate()
    let token = gate.beginRefresh()
    #expect(gate.current == token)
    #expect(gate.isCurrent(token))
}
