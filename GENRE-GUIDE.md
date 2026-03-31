# 🎵 豆瓣音乐收藏 - Genre 标签指南

## 📋 CSV 格式更新

CSV 已新增 `genre` 列，用于记录专辑的音乐流派标签。

### 格式说明

| 列名 | 说明 | 示例 |
|------|------|------|
| title | 专辑名 | Journey in Satchidananda |
| url | 豆瓣链接 | https://music.douban.com/subject/1401325/ |
| date | 标记日期 | 2026-03-23 |
| rating | 评分 | ★★★★★ |
| status | 状态 | 听过 |
| comment | 评论 | 非常非常好听 |
| **genre** | **流派标签** | **Spiritual Jazz\|Jazz\|Eastern** |

### Genre 列规则

- **最多 4 个流派**，用 `|` 分隔
- **新专辑必须标记**，旧专辑可选
- 流派名称尽量使用英文（便于分析）

### 示例

```csv
title,url,date,rating,status,comment,genre
Journey in Satchidananda,https://...,2026-03-23,★★★★★,听过，非常非常好听,Spiritual Jazz
Silent Hill OST,https://...,2026-03-23,★★★★★,听过，很好，Game OST
Thembi,https://...,2026-03-24,★★★★★,好听,Spiritual Jazz|Jazz|Free Jazz
```

## 🏷️ 常用流派标签

### 爵士类
- `Spiritual Jazz` - 精神爵士
- `Hard Bop` - 硬波普
- `Post Bop` - 后波普
- `Modal Jazz` - 调式爵士
- `Free Jazz` - 自由爵士
- `Avant-Garde Jazz` - 先锋爵士
- `Jazz Guitar` - 爵士吉他

### 电子/氛围类
- `Dark Ambient` - 黑暗氛围
- `Electronic` - 电子
- `Ambient` - 氛围

### 其他
- `Dub` - 达布
- `Game OST` - 游戏原声
- `Soundtrack` - 影视原声
- `Rock` - 摇滚
- `Classical` - 古典

## 🚀 快速添加新专辑

### 方式 1：使用脚本

```bash
cd /home/admin/openclaw/workspace

# 完整格式
python3 add-album.py "专辑名" "URL" "★★★★★" "评论" "流派 1|流派 2"

# 示例
python3 add-album.py "Thembi" "https://music.douban.com/subject/1439822/" "★★★★★" "很好" "Spiritual Jazz|Jazz"
```

### 方式 2：直接编辑 CSV

```bash
# 用编辑器打开
code douban-music-4428030.csv

# 或在末尾追加
echo '"Album Name",https://...,2026-03-24,★★★★★,听过，很好,Spiritual Jazz|Jazz' >> douban-music-4428030.csv
```

## 📊 分析推荐

运行推荐工具会自动分析 genre 标签：

```bash
./recommend-music.sh
```

推荐优先级基于你标记的流派分布。

## 💡 标记建议

1. **第一优先级**：最核心的流派（必填）
2. **第二优先级**：次要风格
3. **第三/四优先级**：融合元素或子流派

**示例**：
- Alice Coltrane - Journey in Satchidananda → `Spiritual Jazz|Jazz|Eastern|Modal`
- Akira Yamaoka - Silent Hill OST → `Game OST|Dark Ambient|Industrial`
