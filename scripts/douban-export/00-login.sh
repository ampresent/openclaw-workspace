#!/bin/bash
# 豆瓣登录辅助脚本
# 运行后会打开浏览器，请手动登录豆瓣

set -e
cd /home/admin/openclaw/workspace

AGENT_BROWSER="/home/admin/.nvm/versions/node/v24.14.0/lib/node_modules/agent-browser/bin/agent-browser-linux-x64"
export AGENT_BROWSER_EXECUTABLE_PATH="/usr/bin/google-chrome"

echo "=== 豆瓣登录辅助 ==="
echo ""
echo "正在打开豆瓣登录页面..."
echo "请在浏览器中完成登录"
echo ""
echo "登录后，关闭浏览器，然后运行："
echo "  bash scripts/douban-export/01-verify-access.sh"
echo ""

# 打开登录页面（交互式）
$AGENT_BROWSER open "https://accounts.douban.com/passport/login?redir=https://music.douban.com/mine?status=collect"

echo ""
echo "浏览器已打开，请登录豆瓣..."
echo "按 Enter 键继续验证..."
read

# 验证登录状态
echo "验证登录状态..."
$AGENT_BROWSER open "https://music.douban.com/mine?status=collect"
sleep 2

TITLE=$($AGENT_BROWSER get title)
echo "页面标题：$TITLE"

if echo "$TITLE" | grep -qi "登录"; then
    echo "⚠️  仍未登录，请检查是否登录成功"
else
    echo "✓ 登录成功！"
fi
