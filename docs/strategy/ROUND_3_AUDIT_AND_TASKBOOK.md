# Round 3 审计报告与任务书

> **版本**：v0.3 / Route B 延续  
> **日期**：2026-06-22  
> **用途**：上一轮改动审计、遗留 bug 登记、用户走查问题清单、Round 3 开发任务与 PR 切分。  
> **执行顺序**：先 P0 → P1 → P2；P3 长期 backlog。

---

## 一、上一轮改动审计

### 1.1 完成度对照

| 任务 | 状态 | 备注 |
| --- | --- | --- |
| WordbookDetailView 三态（home/session/done/manage） | ✅ | 有小 bug，见 §2 |
| WordbookProgressCardView | ✅ | 数字正确，UI 朴素 |
| WordbookManageView 词库管理 + CSV | ⚠️ | 分页 + 编辑器有 bug |
| WordbookSessionPlanner 自适应混合会话 | ✅ | 逻辑正确，等同 TechWordPet |
| WordbookStore 每日计划 / streak / snapshot | ✅ | `daily_review_log`，schema 升级正确 |
| AppIndex 模糊+拼音+别名 | ✅ | 30 个别名，CFStringTransform pinyin |
| AppAliasTable + PinyinIndex | ✅ | 实现简洁 |
| LauncherBridge 物理删除 | ✅ | 替代为 `ModuleLauncherHooks` |
| Clipboard 时间分组 + 3 segment | ✅ | All / Pinned / Image |
| Todo 三 tab + placeholder | ✅ | |
| Settings SwiftUI 重写 | ✅ | |
| LauncherRootView 拆分 | ✅ | 1099 → 390 + controllers |
| Continue · N more 显示数量 | ❌ | 字面 `"Continue · more"`，见 U-2 |
| Settings Modules toggle 防抖 | ❌ | 仍每次 toggle 写盘，见 U-5 |

**构建**：`swift build` 干净；测试 118 项通过（Round 2 末）。

### 1.2 遗留 Bug 登记

| ID | 严重度 | 摘要 | 位置 | Round 3 |
| --- | --- | --- | --- | --- |
| **U-1** | 🔴 | `handleTextChange` 1 字符提示死代码（`guard !isEmpty` 后 `!isEmpty` 永假） | `LauncherRootView.swift` ~328–339 | P0-3 |
| **U-2** | 🔴 | Done 态 `continueButton.title = "Continue · more"` 无真实数字 | `WordbookDetailView.swift` ~352 | P0-4 |
| **U-3** | 🔴 | Manage 无限滚动：`scrollViewDidScroll` 非有效回调，仅前 200 行 | `WordbookManageView.swift` ~301–304 | P0-5 |
| **U-4** | 🔴 | Manage 右键菜单 `target = nil`，三项全无效 | `WordbookManageView.swift` ~121–127 | P0-6 |
| **U-5** | 🟡 | Settings Modules toggle 无 debounce | `SettingsSwiftUIView.swift` ~79–90 | P1-1 |
| **U-6** | 🟡 | Clipboard Image 仅存 `"[Image]"` 文本，无法粘贴图片字节 | Clipboard 模块 | P1-2 |
| **U-7** | 🟡 | `showError`：`subState = .home` 但 `showSubview(.session)` | `WordbookDetailView.swift` ~358–363 | P0-7 |
| **U-8** | 🟡 | `handleKeyDown` caps lock + S 边界（minor） | `WordbookDetailView.swift` | 可忽略 |
| **U-9** | 🟢 | `AppAliasTable.aliases` 忽略 `name`/`zhNames` 参数 | `AppAliasTable.swift` | 接口冗余 |
| **U-10** | 🟢 | `ModuleLauncherHooks.shared` 仍单例，与 `LauncherEnvironment` 双线 | 架构 | P3-1 |

---

## 二、用户报告 Bug + 修复方案

### 2.1 Bug A：Notes Mind Map 打开关不掉（N-1）

**根因**：`NotesDetailSheets.presentMindMap` 用 `beginSheet` 弹出无关闭按钮的窗口；`NotesMindMapView` 无 Done / keyDown；外部无 `endSheet`。

**用户期望**：
- 主区域内 `[Tree | Mind Map]` segment 切换，不弹 sheet
- Map：双击文件夹展开/折叠；双击文件 Typora 打开
- Esc 与 Tree 一致 → 退回 launcher home

**任务**：**P0-1** — 详见 §6.1、`docs/adr/017-notes-inline-mindmap.md`（待建）

### 2.2 Bug B：Wordbook 三按钮语义重构（W-1）

**现状**：Unknown / Fuzzy / Known（TechWordPet SM-1）

**目标**：
| 按钮 | 语义 | 对应 |
| --- | --- | --- |
| 不认识 | progress 归零 | `.unknown` |
| 认识 | progress +1 | `.known` |
| 已学过 | 标记 mastered，不再 due | **新** `.mastered` |
| ~~模糊~~ | 删除 UI；枚举保留兼容旧 DB | `.fuzzy` → 按 `.known` 处理 |

**快捷键**：`1` 不认识 · `2` 认识 · `3` 已学过（左→右）

**已学过**：`gradeCurrent(.mastered)` 直接 `advance()`，跳过 `revealAnswer`

**任务**：**P0-2** — 详见 §6.2、`docs/adr/018-wordbook-three-button-grade.md`（待建）

---

## 三、模拟用户走查 — 问题索引

按模块登记，严重度供排期参考；完整描述见原审计。

| 模块 | 🔴 | 🟡 | 🟢 | 代表项 |
| --- | --- | --- | --- | --- |
| 搜索栏 S-* | 0 | 2 | 3 | S-1 Tab 链、S-4 提示（U-1） |
| 主页 H-* | 0 | 2 | 3 | H-2 卡片 due 徽章 |
| Wordbook W-* | 4 | 11 | 5 | W-1~W-4 = U-2~U-4 + Bug B |
| Notes N-* | 1 | 5 | 3 | N-1 Mind Map |
| Translate T-* | 0 | 3 | 3 | T-2 语种 chip |
| Clipboard C-* | 0 | 4 | 1 | C-1 Image 粘贴 |
| Todo TD-* | 0 | 3 | 2 | TD-1 中文时间（P3-5） |
| Snippets SN-* | 0 | 2 | 1 | SN-1 双击语义 |
| Secrets SE-* | 0 | 1 | 2 | SE-1 locked 隐藏 toolbar |
| Settings ST-* | 0 | 3 | 2 | ST-1 debounce |
| 架构 A-* | 0 | 2 | 2 | A-2 planner 4× DB round-trip |

### 4.1 问题类别摘要

- **易用性**：快捷键不可发现、状态不清、误操作不可恢复、Esc 丢上下文
- **功能性**：Mind Map / Manage 滚动 / 右键 / Image 粘贴完全不工作；文案与逻辑错误
- **UI**：Translate/Todo 顶栏不统一、对比度、空白布局

---

## 五、Round 3 任务书

### 5.1 P0（必须本期完成）

| # | 任务 | 触及文件 | 量级 |
| --- | --- | --- | --- |
| **P0-1** | Notes Mind Map inline toggle；删 `presentMindMap`；toolbar `[Tree\|Map]`；`mouseDown` 双击 | `NotesDetailView.swift`, `NotesMindMapView.swift`, `NotesDetailSheets.swift` | M |
| **P0-2** | Wordbook 三按钮：删 Fuzzy UI，加 Mastered；`ReviewScheduler` + `recordReview`；快捷键 1/2/3；中文按钮 | `ReviewScheduler.swift`, `WordbookStore.swift`, `WordbookDetailView.swift` | M |
| **P0-3** | 修 U-1：`handleTextChange` 1 字符提示 | `LauncherRootView.swift` | XS |
| **P0-4** | 修 U-2：Continue 按钮真实数字 | `WordbookDetailView.swift` | XS |
| **P0-5** | 修 U-3：Manage 无限滚动 `didLiveScrollNotification` | `WordbookManageView.swift` | S |
| **P0-6** | 修 U-4：右键菜单 `items.forEach { $0.target = self }` | `WordbookManageView.swift` | XS |
| **P0-7** | 修 U-7：`showError` 状态与 subview 一致 | `WordbookDetailView.swift` | XS |
| **P0-8** | `NotesImageToolsPanel` 无关闭则加 Done | `NotesImageToolsPanel.swift` | S |
| **P0-9** | Progress card 加载态 `"Loading…"` 非空串 | `WordbookProgressCardView.swift`, `WordbookDetailView.swift` | XS |

### 5.2 P1（强烈建议本期或紧接下期）

| # | 任务 | 量级 |
| --- | --- | --- |
| P1-1 | Settings Modules debounce 200ms | XS |
| P1-2 | Clipboard 图片字节 + schema v3 | L |
| P1-3 | Manage 自定义 sheet 编辑器（替代 NSAlert） | M |
| P1-4 | Reset Stage 二次确认 | XS |
| P1-5 | Manage Stage tooltip + 1/9 (new)/(mastered) | S |
| P1-6 | New Words Only 额度 0 时禁用 + tooltip | XS |
| P1-7 | 进度卡对比度 + 隐藏 0d streak | S |
| P1-8 | Notes filter Esc 智能清空 | XS |
| P1-9 | gear 移除 Mind Map（合 segment） | XS |
| P1-10 | Snippets 双击=使用，Cmd+E=编辑 | S |
| P1-11 | Translate 语种 chip 行 | S |
| P1-12 | Secrets locked 隐藏 toolbar | XS |
| P1-13 | Clear Unpinned / Delete folder 二次确认 | S |
| P1-14 | Settings Wordbook tab | M |
| P1-15 | Latency HUD → Developer tab | XS |
| P1-16 | 卡片 Wordbook/Todo due 徽章 | S |

### 5.3 P2（打磨）

P2-1 … P2-13：Translate 提示/错误 banner/swap 禁用；Clipboard header 不可选；Todo placeholder/count；Snippets trigger 必填；Wordbook 完成态文案；侧栏 +N more；卡片按键缩放等。（见原审计 §5.3）

### 5.4 P3（长期）

P3-1 去掉 `ModuleLauncherHooks.shared` · P3-2 planner 合并 SQL · P3-3 FTS5 · P3-4 Notes 系统默认打开 · P3-5 中文时间 · P3-6 错词本

---

## 六、关键模块开发细则（摘要）

### 6.1 Notes Mind Map inline（P0-1）

- 新增 `NotesViewMode` / `ViewMode`：`outline | mindMap`
- `mindMapScroll` + `mindMapView` 与 `scrollView` 互斥显示
- `NotesMindMapView`：`onActivate`、`mouseDown` 双击、`selectedPath` 高亮
- **删除**：`openMindMap`、`presentMindMap`、gear 中 Mind Map 项
- **注意**：Expand/Collapse 仅 outline 显示；filter 仅 outline；`refreshTree` 后 Map 模式需 `reload`
- **不要**：缩放/pan/关系箭头编辑

### 6.2 Wordbook 三按钮（P0-2）

- `WordFamiliarity.mastered`；`ReviewScheduler.schedule(.mastered)` → stage 顶格 + ~100 年 delay
- `recordReview`：`mastered_at = lastISO`，`familiarity = "mastered"`
- UI：`不认识 | 认识 | 已学过`；bezelColor（macOS 13+）；accessibility labels
- `.fuzzy` 旧数据：`schedule` 回退 `.known`
- Planner 无需改（mastered 词 `next_review_at` 极远 + `mastered_at` 非空）

### 6.3 Manage 无限滚动（P0-5）

```swift
NotificationCenter.default.addObserver(..., name: NSScrollView.didLiveScrollNotification, object: tableScroll)
tableScroll.contentView.postsBoundsChangedNotifications = true
// visibleRect.maxY >= contentHeight - 80 → loadMore()
// guard !isLoading
```

删除无效的 `scrollViewDidScroll(_:)`.

### 6.4 Manage 右键（P0-6）

```swift
menu.items.forEach { $0.target = self }
```

### 6.5 Settings debounce（P1-1）

`Task.sleep(200ms)` + cancel 前一 task → `setEnabledModules`

### 6.6 Clipboard 图片（P1-2，独立 PR）

`ClipboardPayload` text/image；schema v3；v2→v3 迁移；`copyEntry`/`pasteEntry` 按类型写 pasteboard

---

## 七、风险

| 风险 | 等级 | 缓解 |
| --- | --- | --- |
| Mind Map mouseDown vs scroll | M | hitTest 限定 node rect |
| mastered 100 年后 `next_review_at` 字符串比较 | L | 单测 `dueWords` 排除 mastered |
| 删 fuzzy UI 后旧 fuzzy 数据 | L | `.fuzzy` → `.known` 兼容 |
| Clipboard schema v3 | M | 独立 PR + 迁移 |
| 大词库 Map 布局慢 | L | 初始 collapse all，非 expand all |
| bezelColor macOS 12 | L | `#available` gate |

---

## 八、验收 / 测试 / 文档

### 8.1 用户报 bug 验收

**Mind Map**：无 sheet；segment 切换；双击文件夹/文件；Esc 不卡死

**Wordbook 三按钮**：仅三中文按钮；已学过立即跳下一张；不再 due；mastered 计数 +1

### 8.2 手工 QA

写入 `docs/MANUAL_QA_CHECKLIST.md` §Round 3（16 条，见任务书 §8.2 原稿）

### 8.3 新增单测

- `ReviewSchedulerTests.testMasteredCase`
- `WordbookStoreTests.testMasteredExcludedFromDue`
- `WordbookStoreTests.testFuzzyBackwardCompat`
- `WordbookSessionPlannerTests`（newWordsOnly 不返回 review）
- Manage 滚动触发（mock notification）

### 8.4 文档更新清单

| 文档 | 改动 |
| --- | --- |
| `docs/adr/017-notes-inline-mindmap.md` | 新建 |
| `docs/adr/018-wordbook-three-button-grade.md` | 新建 |
| `docs/MANUAL_QA_CHECKLIST.md` | §Round 3 |
| `docs/specs/MODULE_CONTRACT.md` | `WordFamiliarity.mastered` |
| `docs/ROADMAP.md` | v0.3 P0/P1 列 |

---

## 九、不要做的事

- ❌ Mind Map 缩放/pan/箭头编辑
- ❌ 「已学过」确认 dialog / undo
- ❌ Clipboard 图片与其它大改挤同一 PR
- ❌ toolbar 同时塞 Expand/Collapse/Gear/Segment 四组件（Expand/Collapse 仅 outline）
- ❌ Progress card 改 SwiftUI Chart

---

## 十、推荐 PR 切分

| PR | 范围 | 估时 |
| --- | --- | --- |
| **#A** | P0-3,4,7,8,9 + U-3,U-4（小 bug 包） | 半天 |
| **#B** | P0-1 Notes inline Mind Map + ADR-017 | 1 天 |
| **#C** | P0-2 Wordbook 三按钮 + ADR-018 + 测试 | 1 天 |
| **#D** | P1-1,4,6,9,12,13 小 P1 包 | 半天 |
| **#E** | P1-3,5,7,14,15,16 Wordbook/Settings UX | 1–1.5 天 |
| **#F** | P1-8,10,11 Notes/Snippets/Translate | 半天 |
| **#G** | P1-2 Clipboard 图片 schema v3 | 2–3 天 |
| **#H+** | P2 / P3 按需 | — |

**每个 PR**：`swift build && swift test`；对照 `docs/specs/PERFORMANCE.md` keystroke p95。

---

## 附录：文件速查

| 领域 | 主要文件 |
| --- | --- |
| Launcher | `LauncherRootView.swift`, `LumaSearchBar.swift` |
| Wordbook UI | `WordbookDetailView.swift`, `WordbookManageView.swift`, `WordbookProgressCardView.swift` |
| Wordbook 逻辑 | `WordbookStore.swift`, `WordbookSessionPlanner.swift`, `ReviewScheduler.swift` |
| Notes | `NotesDetailView.swift`, `NotesMindMapView.swift`, `NotesDetailSheets.swift`, `NotesImageToolsPanel.swift` |
| Settings | `SettingsSwiftUIView.swift` |
| Clipboard | `ClipboardHistoryStore.swift`, `ClipboardDetailView.swift`, `ClipboardEntryKind.swift` |
| Hooks | `ModuleLauncherHooks.swift`, `LauncherEnvironment.swift` |
