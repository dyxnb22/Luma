# Dashboard Cards

## Goal

Present Luma modules as rounded cards in a Spotlight/Raycast-inspired dashboard with macOS 26/iOS 26-style soft materials, rounded borders, and direct edit controls.

## Behavior

- Each module exposes a card descriptor.
- Cards are draggable.
- Card order and position persist locally in `Application Support/Luma/card-layout.json`.
- Every card has an edit button.
- Cards can be enabled/disabled.
- Cards can be compact, normal, or expanded.

## Visual Direction

- Spotlight/Raycast-like calm density.
- Rounded card corners.
- Native materials, blur, subtle shadow, and high contrast text.
- No heavy web UI and no WebView.

## Data

Persist per-card layout:

- module id.
- position.
- size mode.
- enabled state.
- last edited at.

## Implementation Entry

- Core model: `Sources/LumaCore/Features/FeatureCard.swift`
- Future UI: `Sources/LumaApp/Launcher` or `Sources/LumaApp/Dashboard`
