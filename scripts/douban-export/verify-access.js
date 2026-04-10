#!/usr/bin/env node
// 豆瓣音乐专辑导出工具 - 使用临时用户数据目录 + 导入 Cookies

const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');

const CONFIG = {
    baseUrl: 'https://music.douban.com/mine?status=collect',
    
    // 临时用户数据目录
    userDataDir: path.join(__dirname, '../../temp/chrome-douban-profile'),
    
    // Cookies 文件
    cookiesFile: path.join(__dirname, '../../cookies-douban.json'),
    
    // 输出文件
    outputFile: path.join(__dirname, '../../temp/douban-albums.csv'),
    
    // 反爬虫设置
    baseDelay: 4000,
    delayJitter: 1000,
    maxRetries: 5,
    
    // 失败检测关键词
    failKeywords: ['访问太频繁', '验证', 'captcha', '403', '登录豆瓣'],
};

// 确保目录存在
function ensureDir(dir) {
    if (!fs.existsSync(dir)) {
        fs.mkdirSync(dir, { recursive: true });
    }
}

// 随机延迟
function randomDelay(base, jitter) {
    const delay = base + Math.floor(Math.random() * jitter * 2) - jitter;
    return new Promise(resolve => setTimeout(resolve, delay));
}

// CSV 转义
function escapeCSV(value) {
    if (value === null || value === undefined) return '';
    const str = String(value).trim();
    if (str.includes(',') || str.includes('"') || str.includes('\n')) {
        return '"' + str.replace(/"/g, '""') + '"';
    }
    return str;
}

// 加载 cookies
function loadCookies() {
    const defaultCookies = [
        { name: 'll', value: '108288', domain: '.douban.com', path: '/' },
        { name: 'bid', value: 'vo4bhsnvkKg', domain: '.douban.com', path: '/' },
        { name: 'ck', value: '_eat', domain: '.douban.com', path: '/' },
        { name: 'ap_v', value: '0,6.0', domain: '.douban.com', path: '/' },
    ];
    
    if (fs.existsSync(CONFIG.cookiesFile)) {
        try {
            const fileCookies = JSON.parse(fs.readFileSync(CONFIG.cookiesFile, 'utf-8'));
            console.log(`从文件加载 ${fileCookies.length} 个 cookies`);
            return [...defaultCookies, ...fileCookies];
        } catch (e) {
            console.log('Cookies 文件解析失败，使用默认 cookies');
        }
    }
    
    return defaultCookies;
}

// 检查是否被阻止
async function isBlocked(page) {
    try {
        const title = await page.title();
        const url = page.url();
        return CONFIG.failKeywords.some(keyword => 
            title.toLowerCase().includes(keyword.toLowerCase()) ||
            url.includes('login')
        );
    } catch {
        return false;
    }
}

async function main() {
    console.log('=== 豆瓣音乐专辑导出工具 ===');
    console.log(`时间：${new Date().toLocaleString('zh-CN')}`);
    console.log('');
    
    // 准备临时目录
    ensureDir(CONFIG.userDataDir);
    ensureDir(path.dirname(CONFIG.outputFile));
    
    // 加载 cookies
    const cookies = loadCookies();
    console.log(`加载了 ${cookies.length} 个 cookies`);
    
    // 启动浏览器（临时配置文件）
    console.log('启动浏览器（临时配置文件）...');
    const browser = await chromium.launch({
        headless: false,
        executablePath: '/usr/bin/google-chrome',
        args: [
            '--disable-gpu',
            '--no-sandbox',
            '--disable-dev-shm-usage',
        ],
    });
    
    const context = await browser.newContext({
        userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
        viewport: { width: 1280, height: 800 },
        cookies: cookies.map(c => ({
            ...c,
            url: `https://${c.domain.replace(/^\./, '')}/`,
        })),
    });
    
    const page = await context.newPage();
    
    try {
        // 导航到页面
        console.log('打开豆瓣音乐页面...');
        await page.goto(CONFIG.baseUrl, { waitUntil: 'networkidle', timeout: 30000 });
        await randomDelay(3000, 500);
        
        // 检查登录状态
        const title = await page.title();
        console.log(`页面标题：${title}`);
        
        if (title.includes('登录')) {
            console.log('');
            console.log('⚠️  未检测到登录状态');
            console.log('');
            console.log('原因：你提供的 cookies 可能不完整或已过期。');
            console.log('');
            console.log('解决方案：');
            console.log('1. 在已登录豆瓣的 Chrome 浏览器中访问：');
            console.log('   https://music.douban.com/mine?status=collect');
            console.log('');
            console.log('2. 按 F12 打开开发者工具 → Application → Cookies');
            console.log('');
            console.log('3. 复制所有 douban.com 的 cookies（特别是 dbcl2、douban-favgate 等）');
            console.log('');
            console.log('4. 粘贴到 cookies-douban.json 文件中');
            console.log('');
            console.log('或者，你可以手动在浏览器中完成导出，我来帮你处理数据。');
            return;
        }
        
        console.log('✓ 已登录');
        console.log('');
        
        // 截图
        await page.screenshot({ path: 'temp/douban-logged-in.png', fullPage: true });
        console.log('截图已保存到：temp/douban-logged-in.png');
        
        // 等待专辑列表
        console.log('');
        console.log('分析页面结构...');
        
        // 尝试多种选择器
        const selectors = [
            '.article-list li',
            '.list li',
            'li.item',
            '[class*="article"] li',
        ];
        
        let albumSelector = null;
        for (const selector of selectors) {
            try {
                const elements = await page.$$(selector);
                if (elements.length > 0) {
                    albumSelector = selector;
                    console.log(`找到专辑列表选择器：${selector} (${elements.length} 个元素)`);
                    break;
                }
            } catch {
                continue;
            }
        }
        
        if (!albumSelector) {
            console.log('未找到标准专辑列表选择器');
            console.log('页面 HTML 片段:');
            const html = await page.content();
            console.log(html.substring(0, 2000));
            return;
        }
        
        // 获取第一页专辑数量
        const albums = await page.$$(albumSelector);
        console.log(`第一页找到 ${albums.length} 张专辑`);
        
        // 检查分页
        const nextPage = await page.$('link[rel="next"], .next a, a.next');
        const hasNextPage = nextPage !== null;
        console.log(`有分页：${hasNextPage}`);
        
        console.log('');
        console.log('✓ 页面结构分析完成');
        console.log('');
        console.log('准备开始导出...');
        console.log(`预计导出 ${albums.length * (hasNextPage ? '多' : '1')} 页数据`);
        console.log('');
        console.log('下一步：运行完整导出脚本');
        console.log('  node scripts/douban-export/export-albums.js');
        
    } catch (e) {
        console.error('错误:', e.message);
        console.error(e.stack);
    } finally {
        console.log('');
        console.log('浏览器保持打开状态，10 秒后自动关闭...');
        await randomDelay(10000, 0);
        await browser.close();
    }
}

main().catch(console.error);
