# Round 1 Findings

### F-1-01 Global search shows zero result rows (Quicklinks / Apps)
- 场景: `gh swift package`, `Safari`
- 复现: Hotkey → 输入 `gh swift package` 或 `Safari` → 等待 2s → 截图
- 预期: 出现 GitHub Search 行或 Safari 应用行
- 实际: 搜索框文本正确，但结果列表区域空白；Return 显示 “No results yet”
- 截图: `screenshots/gh-swift-02-results.png`, `screenshots/final-Safari.png`
- 严重度: P1（功能失效）
- 假设原因: 自动化输入未稳定触发 `controlTextDidChange`，或 snapshot sequence 与 deliver 竞态
- 修复路径: `LauncherRootController.handleTextChange`, `LumaSearchBar.queryText`, `scripts/qa/paste_query.swift`

### F-1-02 Kill Process 无进程列表
- 场景: `kill preview`
- 复现: Hotkey → 输入 `kill preview`
- 预期: 列出 Preview 等 GUI 进程
- 实际: 仅显示命令提示，无进程行
- 截图: `screenshots/kill-preview-02-results.png`
- 严重度: P1（功能失效；也可能 Preview 未运行）
- 假设原因: 同 F-1-01 的结果列表未渲染，或 Preview 未启动
- 修复路径: `KillProcessModule.handle`, launcher 结果渲染链

### F-1-03 Menu Bar Search 无菜单项
- 场景: Cursor 前台时 `mb fold`
- 复现: Hotkey → `mb fold`
- 预期: 出现 Fold 相关菜单项
- 实际: 仅命令提示，无菜单行
- 截图: `screenshots/mb-fold-02-results.png`
- 严重度: P1
- 假设原因: 菜单缓存未就绪或结果列表未渲染
- 修复路径: `MenuItemsModule`, `MenuBarTreeService`

### F-1-04 会话恢复污染 home 截图
- 场景: 冷启动 smoke `home-01`
- 复现: 连续跑多条 smoke 后查看 `home-01-home.png`
- 预期: 空搜索框 + Open Apps
- 实际: 搜索框残留 `cligh swift package` 等旧查询
- 截图: `screenshots/home-01-home.png`
- 严重度: P2（体验/测试可靠性）
- 假设原因: `restoreLastSessionIfNeeded` 在 panel 打开时恢复上次查询
- 修复路径: `scripts/qa/drive.sh` open 动作清空字段（已加 AX clear）

### F-1-05 中文输入法干扰早期 type 脚本
- 场景: 初始 `drive.sh type` 使用剪贴板/keystroke
- 复现: 拼音输入法下输入 `gh`
- 预期: `gh swift package`
- 实际: `规划 swiftpackage` 等乱码
- 截图: `screenshots/verify-gh-14-prefix-keystroke.png`
- 严重度: P2（已用 ABC + keycode 缓解）
- 假设原因: 系统默认 Pinyin 键盘
- 修复路径: `scripts/qa/paste_query.swift` 内 `TISSelectInputSource(ABC)`
