# 来源加权初始名单（2026-08-12 整理）

> 配套：[`RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md`](RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md)
> （ADR-0010 §3.7 条 12，FUS-SOURCE-WEIGHTING）。
> 本文件是来源加权机制的**初始种子名单**；实现时按此整理为配置文件/env 注入的
> 域名列表与 URL 形态规则，可增删，**不硬编码**。
> 档位：白名单 1.1（政府/机关单位，直接采纳）；白名单外默认 1.0；劣质源 0.7。

## 1. 白名单（1.1）：政府与机关单位

### 1.1 后缀规则（自动匹配）

| 规则 | 范围 |
|---|---|
| `*.gov.cn` | 中国各级政府、部委、局办官网 |
| `*.gov.hk` / `*.gov.mo` / `*.gov.tw` | 港澳台政务网站 |
| `*.gov` | 其他国家/地区政府域名（如美国联邦/州政府） |
| `*.edu.cn` | 高校/科研教育机构主站（事业单位；排除个人主页/博客子域） |
| `*.ac.cn` | 中科院与科研院所 |
| `*.mil.cn` / `*.mil` | 军队/国防官方域名 |

### 1.2 显式机构（非上述后缀的政府间/机关单位）

| 域名 | 机构 |
|---|---|
| `un.org` | 联合国 |
| `who.int` | 世界卫生组织 |
| `imf.org` | 国际货币基金组织 |
| `worldbank.org` | 世界银行 |
| `oecd.org` | 经济合作与发展组织 |
| `wto.org` | 世界贸易组织 |
| `nato.int` | 北约 |
| `europa.eu` | 欧盟 |
| `cas.cn` / `cass.cn` / `cae.cn` | 中国科学院 / 中国社会科学院 / 中国工程院 |

说明：其余机关事业单位（医院、图书馆、行业协会等）若不在上述后缀，实现时按官方站
补充显式清单；未知机构站一律不进白名单。

## 2. 劣质源（0.7）

### 2.1 平台级域名/URL 形态（机械识别）

| 平台 | 域名/URL 形态 |
|---|---|
| CSDN | `csdn.net`（含 `blog.csdn.net`） |
| 知乎 | `zhihu.com`（含 `zhuanlan.zhihu.com` 专栏） |
| 百家号 | `baijiahao.baidu.com/s?id=...` |
| B 站个人专栏 | `bilibili.com/read/cv*`、`bilibili.com/opus/*`；`b23.tv` 短链展开后判定 |
| 微信公众号 | `mp.weixin.qq.com`（平台默认 0.7；官方机构号由模型判断例外——待确认） |
| 头条/头条号 | `toutiao.com`（文章页） |
| 简书 | `jianshu.com` |
| 网易号 | `dy.163.com` |
| 搜狐号 | `mp.sohu.com` |
| 企鹅号 | `om.qq.com` |
| 大鱼号 | `mp.dayu.com` |

### 2.2 独立新闻/财经媒体种子清单（0.7）

- 科技/商业媒体：`36kr.com`、`huxiu.com`、`tmtpost.com`、`ifanr.com`、
  `pingwest.com`、`ithome.com`、`cnbeta.com`、`donews.com`、`leiphone.com`、
  `jiqizhixin.com`、`qbitai.com`；
- 财经资讯/社区：`eastmoney.com`、`xueqiu.com`、`10jqka.com.cn`、
  `stockstar.com`、`hexun.com`、`caixin.com`；
- 商业新闻门户（仅新闻/财经频道子域）：`news.sina.com.cn`、`finance.sina.com.cn`、
  `news.163.com`、`money.163.com`、`news.qq.com`、`finance.qq.com`、
  `news.sohu.com`。

说明：0.7 是**标注/降权档，不是屏蔽**；专业媒体（如财新）如后续需要升档，另行调整。

### 2.3 账号级规则（第三层模型判断）

- 自媒体新闻号：名称/认证呈"XX新闻/资讯/快讯"且非官方媒体；
- 财经号："XX财经/财富/投资/观察"类账号；
- 个人号/非认证号：无平台认证标识；
- 小站兜底：站点主体不明、无机构信息、以聚合转载为主 → 0.7。

## 3. 待确认项

1. 官方媒体（`xinhuanet.com`、`people.com.cn`、`cctv.com`、`cnr.cn` 等）是否入
   白名单或设独立升档？（当前按"只放政府/机关单位"未入）
2. `mp.weixin.qq.com` 默认劣质档是否接受（官方机构公众号会一并标 0.7，由模型判断
   例外）？
3. `edu.cn` 个人主页（`~user` 路径、个人博客子域）的排除规则。

## 4. 落地说明

- 机械层落地：§1 后缀规则 + §1.2/§2.1/§2.2 显式域名列表，配置文件/env 注入；
- 模型层落地：§2.3 规则写入检索子代理加权判断提示词；
- 本文件只作种子，实际名单以配置为准，允许运行时增删。
