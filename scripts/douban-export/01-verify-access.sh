#!/bin/bash
# 验证豆瓣音乐页面访问 - 使用 Chrome 用户数据目录

set -e
cd /home/admin/openclaw/workspace

# 设置 agent-browser 路径
AGENT_BROWSER="/home/admin/.nvm/versions/node/v24.14.0/lib/node_modules/agent-browser/bin/agent-browser-linux-x64"

# 使用系统 Chrome 浏览器和用户数据目录
export AGENT_BROWSER_EXECUTABLE_PATH="/usr/bin/google-chrome"
CHROME_USER_DATA="$HOME/.config/google-chrome"

echo "=== 豆瓣音乐页面访问验证 ==="
echo "时间：$(date)"
echo "Chrome 用户数据目录：$CHROME_USER_DATA"
echo ""

# 打开豆瓣音乐页面（使用现有 Chrome 用户数据，包含登录状态）
echo "正在打开豆瓣音乐页面（使用你的 Chrome 登录状态）..."

# 注意：使用用户数据目录时需要关闭所有 Chrome 实例
# 或者使用 --remote-debugging-port 连接已有浏览器

# 方法 1: 尝试直接打开（可能需要先关闭 Chrome）
$AGENT_BROWSER open "https://music.douban.com/mine?status=collect" \
    --executable-path "$AGENT_BROWSER_EXECUTABLE_PATH" \
    --args "--user-data-dir=$CHROME_USER_DATA"

echo "等待页面加载..."
sleep 3

# 获取页面标题
echo "获取页面标题..."
TITLE=$($AGENT_BROWSER get title)
echo "页面标题：$TITLE"

# 截图验证
echo "正在截图..."
$AGENT_BROWSER screenshot temp/douban-verify-screenshot.png

# 检查是否登录
echo "检查登录状态..."
$AGENT_BROWSER snapshot -i > temp/douban-page-snapshot.txt

# 检查是否包含登录提示
if grep -qi "登录\|login" temp/douban-page-snapshot.txt; then
    echo "⚠️  检测到登录提示，可能未登录"
    echo ""
    echo "页面快照内容:"
    head -30 temp/douban-page-snapshot.txt
    echo ""
    echo "提示：如果 Chrome 正在运行，可能需要先关闭 Chrome 才能使用用户数据目录。"
else
    echo "✓ 页面访问正常 - 已登录"
fi

echo ""
echo "=== 验证完成 ==="
echo "截图：temp/douban-verify-screenshot.png"
echo "快照：temp/douban-page-snapshot.txt"
