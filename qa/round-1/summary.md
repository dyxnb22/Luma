# Round 1 Summary

## 测了什么
- 构建并签名 `build/Luma.app`，`swift test` 357 tests 全绿
- 建立 `scripts/qa/drive.sh` + `paste_query.swift` 截图驱动链（含 ABC 键盘切换、点击聚焦、query sync）
- 跑完 `run_round1_smoke.sh` 全量 trigger smoke（历史 12 + 新 4 模块）
- 抽样阅读截图：home、gh、mb、kill、proj

## 结果
| 类别 | 通过 | 失败 |
|------|------|------|
| Hotkey 打开面板 | ✓ | |
| Home Open Apps 区 | ✓ | |
| 命令提示条（clip/mb/kill/proj） | ✓ | |
| 全局搜索结果行 | | ✗ |
| Quicklinks gh 行 | | ✗ |
| Kill 进程行 | | ✗ |
| Menu 菜单行 | | ✗ |

## Findings 统计
| P0 | P1 | P2 | P3 |
|----|----|----|-----|
| 0  | 3  | 2  | 0  |

## 性能
- 本轮未跑 `doctor` keystroke→paint（待 Round 2 修复 P1 后补测）

## 下一步（Round 2）
1. 修复全局搜索结果不渲染（优先）
2. `drive.sh open` 清空会话查询（已落地）
3. 重跑全量 smoke + doctor 性能截图
