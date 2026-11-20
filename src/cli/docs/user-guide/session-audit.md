# session audit 使用指南

`qtcloud-agent session audit` 是发布前的检查口：把一份会话或一个目录扫一遍，按六类敏感信息计数，**只读**——不改一个字节，也不替你打码。命中就退出码 1，退出码本身就是结论，可以直接当脚本和 CI 的门禁用。

典型流程是两头各跑一次：拿到原始会话先跑一遍看有多少处，洗完再跑一遍看是否归零。

## 用法

```sh
qtcloud-agent session audit <路径>          # 目录或单个文件
qtcloud-agent session audit <路径> --json   # 机器读
```

目录会递归往下找；给单个文件就只扫这一个。路径不存在直接报错并退出码 3。

配合 `session list` 找目标很顺：`session list --json` 的 `origin` 字段就是每个会话的存储位置，原样传给 `audit` 即可。

```sh
qtcloud-agent session list --json | jq -r '.sessions[0].origin'
qtcloud-agent session audit <上一步的输出>
```

## 扫描范围

只扫这几类扩展名的文本文件：`json`、`jsonl`、`md`、`txt`、`log`、`yaml`、`yml`。读不到的文件跳过，不计数也不报错。

传单个文件时不看扩展名，直接读。所以把 `.zstd` 单独丢给它也会被扫，只是压缩内容读不出明文，照样报 `0 命中`。

**三个盲区**：opencode 的 `~/.local/share/opencode/opencode.db`、hermes 的 `~/.hermes/state.db` 两个 SQLite 库，以及 dsh 的 `session.jsonl.zstd` 压缩会话。它们里面同样存着路径、域名和历史消息，但本机 47 个 opencode 会话、89 个 hermes 会话、89 个 dsh 会话目前一个都不会被扫到。

## 六类命中

- `secrets`：密钥形态。`sk-`、`ghp_`、`AKIA`、`LTAI`、`xox` 等前缀，飞书 webhook，私钥块，`Bearer` 令牌；
- `emails`：邮箱地址；
- `names`：人名，当前登记 `Guo Zhang` 与 `zhangguo`；
- `hosts`：部署域名。`quanttide.com`、`quanttide.top` 及子域，`oss-cn-*.aliyuncs.com`；
- `buckets`：桶名与对象存储地址。`oss://` 后的桶名，以及量潮已知的 11 个桶名；
- `paths`：家目录路径 `/home/iguo`。

## 输出与退出码

人读的输出按文件列命中，最后给总数：

```
── 审计 ──  /tmp/audit-what（1 个文件）
  /tmp/audit-what/demo.md  buckets 5, emails 1, hosts 3, names 3, paths 1, secrets 6

合计 19 处，涉及 1 个文件
```

一个文件都没命中时只打一行 `0 命中`。`--json` 输出 `{path, files, total, totals, hits}`，`hits` 是逐文件的分类计数，便于程序判断该拦哪一份。

- 0：0 命中；
- 1：有命中；
- 3：路径不存在。

注意目录里一个白名单文件都没有时打的是 `0 个文件`，退出码同样是 0，和「扫了且干净」目前无法区分。

## 实测

本目录一个已结束的原始 pi 会话（234 KB）跑出来 353 处：`paths` 351、`emails` 1、`hosts` 1，退出码 1——家目录路径铺满了整份记录。

同一份材料洗完放进实验室的 `data/`，31 个文件跑出来 `0 命中`，退出码 0。所以「归零」是可验证的，不用靠眼睛看。

## 局限

- 没扫到也报干净：目录里没有白名单文件时返回 0，与「扫了且干净」无法区分，当门禁用会放行从未被检查的数据；
- 只报不清：它不修改任何文件，打码是另一件事；
- 扫不到的比扫得到的多：本机 592 个会话里只有 pi 的 367 个在覆盖内，opencode 与 hermes 存在 SQLite 库里，dsh 是压缩文件；
- 处数是命中次数，不是位置数：同一个词可能被两条规则各计一次，例如 `oss://qtclass-studio` 同时命中桶名规则和 `oss://` 规则；
- 清单是写死的：域名、桶名、人名、密钥前缀都按量潮的已知情况登记，换个平台的新形态它不会自己发现；
- 正则不支持前后瞻：占位符 `oss://bucket` 曾被 `oss://` 规则误判，靠精确排除修掉，换了占位符写法要同步改排除项。

## 修改量潮清单

清单集中在 `src/cli/src/audit.rs` 顶部：`BUCKETS` 常量是桶名清单，`rules()` 函数是全部正则，`rules()` 末尾的排除项管占位符。新增域名、桶名、人名改这两处即可。
