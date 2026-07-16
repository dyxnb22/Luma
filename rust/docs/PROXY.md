# `luma.proxy`

`luma.proxy` 是本机 Mihomo/Clash Verge 的控制面模块。它只连接 loopback 或配置的 Unix
socket，不负责启动代理核心，也不执行导入 Profile 中的脚本、Merge、TUN 或 DNS/rule
编辑逻辑。

## 查询与动作

| Query | 作用 |
| --- | --- |
| `proxy` | 查看 Mihomo 状态、当前 Profile、模式、端口和系统代理 |
| `proxy group <name>` | 查看代理组及节点，选择节点不显示原始凭据 |
| `proxy global` / `proxy rule` | 切换 Mihomo 模式，使用 `PATCH /configs` |
| `proxy profile` | 列出 Luma Profile 和现有 Clash Verge Profile |
| `proxy profile <name>` | 按名称筛选 Profile |
| `proxy profile refresh` | 只刷新 Luma 管理的订阅 Profile |
| `proxy import <source>` | 导入 HTTPS/loopback HTTP 订阅或本地 YAML/节点列表 |
| `proxy refresh` | 刷新 Mihomo Proxy Provider；没有 Provider 时返回 `not_configured` |

Profile 的 `Use`、`Delete` 等写操作需要确认。Clash Verge 中非 Luma-owned Profile 只读，
Merge/Script Profile 不会被修改或执行。

## Profile 导入格式

导入器优先接受 Clash YAML；同时支持常见的：

- Base64 编码的 Clash YAML 或节点列表
- `vless://`
- `vmess://`
- `ss://`
- `trojan://`

节点 URI 会在 macOS adapter 内转换为受控 Clash YAML，原始 URL、token、UUID、密码不会进入
SearchItem、Preview、Action payload、日志或错误消息。未知格式会明确报错，不会执行外部脚本。

YAML 根节点是严格 allowlist：只接受 `name`、`proxies`、`proxy-groups`、
`proxy-providers`、`rule-providers`、`rules` 和 `sub-rules`。任何其他根设置（包括
Controller、secret、listeners、bind-address、端口、DNS、TUN、allow-lan 或 mode）都会在
持久化前被拒绝，因此导入内容不能借由 Luma 的 Profile 源文件覆盖运行时或网络设置。

订阅只允许 HTTPS，或显式 loopback 的 HTTP。HTTPS 最多跟随 3 次且仅限 HTTPS 重定向；
loopback HTTP 不接受重定向。下载以流式 512 KiB 上限读取，因而也会限制 chunked 或没有
`Content-Length` 的响应；请求不会读取用户的 curl 配置，且订阅地址不进入进程参数。

## 应用与回滚

导入保存到：

```text
~/Library/Application Support/LumaNext/proxy-profiles/
```

其中：

- `profiles.json` 只保存 Profile 元数据和 opaque ID
- Profile 源文件与备份使用 `0600`
- Profile 目录使用 `0700`
- 订阅 URL 只保存在 Luma Keychain 引用下

应用 Profile 时，runtime 不会直接接受完整导入 YAML。Luma 先读取当前可信 Mihomo 配置，
只替换 `proxies`、`proxy-groups`、`proxy-providers`、`rule-providers`、`rules` 和
`sub-rules`，保留 Controller、secret、端口、allow-lan、TUN、监听器和 bind 配置。相同的
根节点 allowlist 也适用于写入 Clash Verge 的 Luma-owned 源文件。

导入和订阅刷新会把源文件、`profiles.json` 和 Keychain 中的订阅引用视为一个事务。失败时会
恢复原文件和原 metadata，并把订阅引用恢复为旧值；如果本次此前没有引用，则移除新建引用。
已注册的 Clash Verge 源文件同步失败时，同样恢复这些状态，不会保留半更新的 Profile。

应用过程分为：

1. 写入 Luma Profile 源文件
2. 写入 Luma-owned Clash Verge metadata/源文件
3. 应用 Mihomo runtime

任一步失败都会尽可能恢复原文件、原 Profile metadata 和 current UID；如果恢复也失败，
会明确返回 rollback failure。Profile refresh 只更新本地/已注册源文件，不自动应用 runtime，
需要再次执行 `proxy profile <name>` 的 `Use`。

## 系统代理安全边界

Luma 只管理 HTTP 和 SOCKS 的 loopback 设置，并只在 Luma 上次写入的值仍然匹配时恢复原值。
如果当前网络服务启用了 HTTP/SOCKS 认证、Secure Web Proxy、PAC URL 或 Proxy Auto Discovery，
Luma 返回 `conflict`，不会接管该服务，也不会尝试回滚这些设置。启用时若 Mihomo 只提供
`mixed-port`，该端口会同时用于 HTTP 和 SOCKS；整个操作仍需确认。

## Clash Verge 兼容

默认只读取和有限写入：

```text
~/Library/Application Support/io.github.clash-verge-rev.clash-verge-rev/profiles.yaml
```

`current` UID 会映射到 `items[].name`。现有 Profile 的 YAML 如果可安全读取，会统计节点、
代理组和规则；Merge/Script 或无法读取的文件显示 `metadata unavailable`。写回只针对
Luma-owned local Profile，保留未知字段，不删除用户 Profile。非 Luma-owned Clash Verge UID
只在 adapter 内部用于读取状态，不会出现在 SearchItem ID、Action payload 或界面中。

## Controller 配置

默认 endpoint：

```text
Unix: /tmp/verge/verge-mihomo.sock
TCP:  127.0.0.1:9097
```

可通过设置配置：

```bash
cargo run -p luma -- config set --proxy-controller-unix-socket /path/to/mihomo.sock
cargo run -p luma -- config set --proxy-controller-address 127.0.0.1:9097
cargo run -p luma -- config set --proxy-controller-secret-account mihomo-controller
cargo run -p luma -- config set --proxy-network-service Wi-Fi
```

Controller secret 只接受 Keychain account 名称，不接受或持久化 secret 明文。非 loopback TCP
地址会被拒绝。Controller 请求失败对 UI 只返回通用操作/实体说明，不回显请求路径、代理组或节点
标签。

## 明确不支持

- 自动执行 Script/JavaScript
- Merge 规则编辑或执行
- TUN、DNS、Rule 编辑器
- 多核心管理和后台 daemon
- VLESS/VMess 等协议的完整高级参数编辑
- 自动修改最终生成的 `clash-verge.yaml`
