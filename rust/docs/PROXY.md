# `luma.proxy`

`luma.proxy` 是本机 Mihomo/Clash Verge 的控制面模块。它只连接 loopback 或配置的 Unix
socket，不负责启动代理核心，也不执行导入 Profile 中的脚本、Merge、TUN 或 DNS/rule
编辑逻辑。

## 查询与动作

| Query | 作用 |
| --- | --- |
| `/proxy ` | 设置型首页：系统代理、代理模式、当前节点、配置方案、连接检查和运行信息 |
| `/proxy mode` | 显式列出 Rule（分流）和 Global（全局），并标记当前选项 |
| `/proxy group <name>` | 只显示代理组节点；未选节点 Enter 选择，已选节点 Enter 执行按需 **Test Latency** |
| `/proxy global` / `/proxy rule` | 切换 Mihomo 模式；Global 会先确保 `GLOBAL` selector 经 `PROXY` 转发 |
| `/proxy profile` | 列出 Luma Profile 和现有 Clash Verge Profile |
| `/proxy profile <name>` | 按名称筛选 Profile |
| `/proxy profile refresh` | 只刷新 Luma 管理的订阅 Profile |
| `/proxy import <source>` | 导入 HTTPS/loopback HTTP 订阅或本地 YAML/节点列表 |
| `/proxy sync` | 读取约定 `proxy.yaml`，编译并保存为固定 Luma-owned Profile（不自动应用） |
| `/proxy apply` | 一步编译、保存并应用 `proxy.yaml` 到当前运行中的 Mihomo |
| `/proxy refresh` | 刷新 Mihomo Proxy Provider；没有 Provider 时返回 `not_configured` |

Profile 的 `Use`、`Delete`、约定 `Sync` 等写操作需要确认。Clash Verge 中非 Luma-owned Profile 只读，
Merge/Script Profile 不会被修改或执行。

## 约定配置 `proxy.yaml`

固定路径（可用 `LUMA_NEXT_SUPPORT_DIR` 覆盖根目录）：

```text
~/Library/Application Support/LumaNext/proxy.yaml
```

用户或 Agent 只填写 VPS 必要字段；Luma 用 preset 展开为完整 Mihomo Profile。示例：

```yaml
kind: luma-proxy
version: 1
name: Personal VPS

nodes:
  - name: US VPS
    preset: vless-reality
    server: 203.0.113.10
    port: 443
    uuid: 00000000-0000-0000-0000-000000000000
    sni: www.microsoft.com
    public-key: example-public-key-base64url-xxxxxxxx
    short-id: ab12cd34
```

### Preset

| Preset | 必填 | 自动约定 |
| --- | --- | --- |
| `ss` | server、port、cipher、password | `udp: true` |
| `trojan-tls` | server、port、password；IP 时必填 sni | TLS、TCP、`skip-cert-verify: false` |
| `vless-tls` | server、port、uuid；IP 时必填 sni | TLS、TCP、`encryption: ""` |
| `vless-reality` | server、port、uuid、public-key、short-id（可为空）；IP 时必填 sni | Reality、Vision、Chrome 指纹、TCP |
| `vless-ws-tls` | server、port、uuid、host、path；IP 时必填 sni | TLS、WebSocket |
| `vless-grpc-tls` | server、port、uuid、service-name；IP 时必填 sni | TLS、gRPC |
| `hysteria2` | server、port、password；IP 时必填 sni；可选 obfs/obfs-password、ports | 证书校验开启 |
| `tuic-v5` | server、port、uuid、password；IP 时必填 sni | TUIC v5、安全默认值 |

若 `server` 是域名且省略 `sni`，则默认 `sni = server`。绝不自动设置 `skip-cert-verify: true`。
未知字段直接报错。高级传输（XHTTP、SS plugin、TUIC v4、链式代理、自定义分流等）继续用原生 Mihomo YAML。

### 编译规则

- 生成单一 `PROXY` select 组；节点顺序与 `proxy.yaml` 一致；组内不含 `DIRECT`
- 未配置 `routing` 时保持简单默认：规则仅 `MATCH,PROXY`
- 重复 `/proxy sync` 更新固定 Profile ID `p-c0ffee0000000000000001`，不产生重复项
- Sync 只保存（**compiled**）；应用仍需 Profile 的 **Use**（**applied** / Mihomo 拒绝为 **rejected**）
- 无效 recipe（**invalid**）不写任何 Profile

日常流程：编辑 `proxy.yaml` → `/proxy apply` → `/proxy group PROXY` 查看或切换节点。
需要先检查草稿时仍可用 `/proxy sync` → `/proxy profile` → Use。不做文件监控或后台
daemon。节点 **Test Latency** 通过 Controller 和通用 204 地址探测传输延迟，不后台轮询；
它不能证明某个具体网站不会按 VPS IP、地区或风控策略拒绝访问。

`/proxy` 首页参考常见 Clash 客户端的设置心智模型，但保持键盘式列表交互：

1. **System Proxy**：ON / OFF / OTHER，Enter 执行当前可用的接管或关闭动作
2. **Proxy Mode**：Enter 进入两项选择页，显式显示 Rule（分流）/ Global（全局）和当前 Selected
3. **Current Node**：显示 `PROXY` 当前节点，Enter 进入纯节点列表
4. **Configuration**：Profile 与约定 `proxy.yaml`
5. **Connection Check**：按需检查网络、DNS、listener 和 Controller
6. **Runtime**：独立/Clash Verge 核心、端口、Provider 刷新和地址复制

设置动作成功后 TUI 会自动重新查询当前页面，不再显示旧状态。Global 模式若发现 Mihomo 的
`GLOBAL` selector 停在 `DIRECT` / `REJECT` 且存在 `PROXY`，会先选择 `PROXY` 再切换模式；
切换失败则恢复原 selector。

### 轻量分流 `routing`

需要“少量目标走 VPS，其余直连”时，可在同一文件增加：

```yaml
routing:
  default: direct
  domain-suffixes:
    - openai.com
    - anthropic.com
  domains:
    - api.example.com
  ip-cidrs:
    - 192.0.2.4/32
```

这三个列表中的目标统一编译到 `PROXY`；最后一条按 `default` 生成 `MATCH,DIRECT` 或
`MATCH,PROXY`。IP 规则自动附加 `no-resolve`，IPv6 自动使用 `IP-CIDR6`。域名、CIDR、重复项、
未知字段和总规则数均严格校验；不支持在约定层注入任意 Mihomo 规则字符串。需要复杂规则提供者、
多策略组或规则脚本时继续使用原生 Mihomo YAML。

## Profile 导入格式

导入器优先接受 Clash YAML；同时支持常见的：

- Base64 编码的 Clash YAML 或节点列表
- `vless://`（经同一套 preset 编译器；REALITY/未知参数 fail-closed，不静默丢弃）
- `vmess://`（基础字段）
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

- `profiles.json` 只保存 Profile 元数据和 opaque ID（不含 UUID/密码/公钥）
- Profile 源文件与备份使用 `0600`
- Profile 目录使用 `0700`
- 约定 `proxy.yaml` 必须是普通文件（拒绝 symlink），sync 前强制为 `0600`
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
2. 默认 Clash Verge 模式下，写入 Luma-owned Clash Verge metadata/源文件
3. 应用 Mihomo runtime，并记录最后成功应用的 Luma Profile

任一步失败都会尽可能恢复原文件、原 Profile metadata 和 current UID；如果恢复也失败，
会明确返回 rollback failure。Profile refresh / convention sync 只更新本地/已注册源文件，
不自动应用 runtime，需要再次执行 `/proxy profile <name>` 的 `Use`。显式配置独立 Mihomo
Controller 时，Profile current 状态由 Luma 的 `profiles.json` 维护，不读取或写入 Clash Verge。

Luma 不负责 Mihomo 进程生命周期。外部 Mihomo 重启后会重新读取它自己的基础配置；若该配置
不会恢复最后应用的节点和规则，需要再次执行 Profile 的 `Use`。`profiles.json` 中的 current
表示 Luma 最后一次成功应用，而不是外部进程重启后的配置证明。

## 系统代理安全边界

Luma 以一个事务管理 macOS HTTP、HTTPS（Secure Web Proxy）和 SOCKS 三项 loopback 设置，
并只在 Luma 上次写入的值仍然匹配时恢复原值。如果当前网络服务启用了 HTTP/SOCKS/HTTPS
认证、PAC URL 或 Proxy Auto Discovery，Luma 返回 `conflict`，不会接管该服务，也不会尝试
回滚这些设置。普通、无认证的 loopback HTTPS 可安全接管。启用时若 Mihomo 只提供
`mixed-port`，该端口会同时用于三项。Enable/Switch 在确认 Mihomo listener 可用后单次 Enter
执行；Disable 仍需确认，以避免意外恢复接管前的旧设置。

状态只有在所有 Mihomo 提供的协议均已启用并且 loopback 地址、端口完全匹配时才显示
`System proxy: LUMA`；全部关闭显示 `OFF`，指向 Clash Verge 或其他端口时显示
`OTHER (host:port)`。OFF 时 Enter 为 Enable；OTHER 时 Enter 为带确认的
**Use Luma**，并在修改前检查目标 listener。Luma 会把当前外部代理保存为恢复点，
所以之后 Disable 会恢复它；未经用户确认不会覆盖一个正在工作的 Clash Verge 代理。

关闭 Clash Verge 窗口或后台核心并不保证 macOS 自动清除原系统代理。如果系统仍指向已经停止
监听的 Clash 端口，首页会明确显示 `OTHER`；此时需执行 **Use Luma**，否则系统流量不会自动
改走独立 Mihomo。

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

两个 Controller endpoint 都未配置时才使用 Clash Verge 的默认 socket + TCP fallback。
只要显式配置 Unix socket 或 loopback TCP，Luma 就只连接显式 endpoint，不会在失败时切换到
另一套代理核心；除非显式 socket 本身就是 Clash Verge 默认 socket，否则同时关闭 Clash Verge
Profile metadata 集成。

Controller secret 只接受 Keychain account 名称，不接受或持久化 secret 明文。非 loopback TCP
地址会被拒绝。Controller 请求失败对 UI 只返回通用操作/实体说明，不回显请求路径、代理组或节点
标签。

## 明确不支持

- 自动执行 Script/JavaScript
- Merge 规则编辑或执行
- TUN、DNS、复杂分流规则编辑器
- 多核心管理和后台 daemon / 文件 watcher
- 完整的 Mihomo 配置语言副本或复杂代理编辑器
- Agent API / 启动或管理 Mihomo 进程
- 自动修改最终生成的 `clash-verge.yaml`
