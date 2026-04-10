#!/usr/bin/env node
// 豆瓣音乐专辑完整导出 - 使用 CDP 连接已有 Chrome 浏览器
// 需要先以调试模式启动 Chrome: google-chrome --remote-debugging-port=9222

import puppeteer from 'puppeteer-core';
import fs from 'node:fs';
import path from 'node:path';

const CONFIG = {
    // 豆瓣用户 ID
    userId: process.env.DOUBAN_USER || '4428030',
    
    // CDP 调试端口
    cdpUrl: process.env.CDP_URL || 'http://127.0.0.1:9222',
    
    // 输出文件
    outputFile: process.env.OUTPUT_FILE || '/home/admin/openclaw/workspace/data/douban-music-2026-03-23.csv',
    
    // 导出类别
    category: 'collect',  // collect=听过，do=在听，wish=想听
    
    // 反爬虫设置
    baseDelay: 3000,
    maxDelay: 6000,
    maxRetries: 5,
};

function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

function randomDelay(min, max) {
    return Math.floor(Math.random() * (max - min + 1)) + min;
}

function csvEscape(str) {
    if (!str) return '';
    const s = String(str).trim();
    if (s.includes(',') || s.includes('"') || s.includes('\n')) {
        return '"' + s.replace(/"/g, '""') + '"';
    }
    return s;
}

async function main() {
    console.log('=== 豆瓣音乐专辑导出工具 (CDP 模式) ===');
    console.log(`用户 ID: ${CONFIG.userId}`);
    console.log(`CDP URL: ${CONFIG.cdpUrl}`);
    console.log(`输出文件：${CONFIG.outputFile}`);
    console.log('');
    
    // 确保输出目录存在
    fs.mkdirSync(path.dirname(CONFIG.outputFile), { recursive: true });
    
    // 连接已有浏览器
    console.log('连接 Chrome 浏览器...');
    let browser;
    try {
        browser = await puppeteer.connect({
            browserURL: CONFIG.cdpUrl,
            defaultViewport: null,
        });
        console.log('✓ 连接成功');
    } catch (e) {
        console.error('连接失败:', e.message);
        console.log('');
        console.log('请先以调试模式启动 Chrome:');
        console.log('  google-chrome --remote-debugging-port=9222 --user-data-dir=/tmp/chrome-debug');
        console.log('');
        console.log('然后重新运行此脚本');
        return;
    }
    
    const pages = await browser.pages();
    const page = pages[0] || await browser.newPage();
    
    // 导航到豆瓣音乐页面
    const baseUrl = `https://music.douban.com/people/${CONFIG.userId}/${CONFIG.category}?sort=time&mode=list`;
    console.log(`打开页面：${baseUrl}`);
    
    await page.goto(baseUrl, { waitUntil: 'networkidle0', timeout: 30000 });
    await sleep(3000);
    
    // 检查登录状态
    const title = await page.title();
    if (title.includes('登录')) {
        console.log('⚠️  未检测到登录状态');
        console.log('请在打开的浏览器窗口中登录豆瓣');
        
        // 等待登录
        for (let i = 0; i < 60; i++) {
            await sleep(5000);
            const newTitle = await page.title();
            if (!newTitle.includes('登录')) {
                console.log('✓ 检测到登录');
                break;
            }
            console.log(`等待登录... (${(i+1)*5}秒)`);
        }
    }
    
    // 截图
    await page.screenshot({ path: '/home/admin/openclaw/workspace/temp/douban-music-page.png', fullPage: true });
    console.log('截图已保存');
    
    // 分析页面结构
    console.log('');
    console.log('分析页面结构...');
    
    const albumSelector = await page.evaluate(() => {
        // 检查各种可能的选择器
        const selectors = [
            '.list-view .item',
            '.article-list .item',
            'li.item',
            '.list li',
        ];
        
        for (const sel of selectors) {
            const count = document.querySelectorAll(sel).length;
            if (count > 0) {
                return { selector: sel, count };
            }
        }
        return null;
    });
    
    if (!albumSelector) {
        console.log('未找到专辑列表选择器');
        const html = await page.content();
        console.log('页面 HTML 片段:');
        console.log(html.substring(0, 2000));
        await browser.disconnect();
        return;
    }
    
    console.log(`找到选择器：${albumSelector.selector} (${albumSelector.count} 个元素)`);
    
    // 开始导出
    console.log('');
    console.log('开始导出...');
    
    const csvHeader = '专辑名称，艺人，用户评分，专辑评分，发行年份，厂牌，听过时间，评论，URL\n';
    fs.writeFileSync(CONFIG.outputFile, csvHeader, 'utf8');
    
    let count = 0;
    let pageNum = 1;
    const maxPages = 250;  // 3515 / 15 ≈ 235 页
    
    while (pageNum <= maxPages) {
        console.log(`\n第 ${pageNum} 页...`);
        
        // 提取当前页数据
        const albums = await page.evaluate((selector) => {
            const items = document.querySelectorAll(selector);
            const results = [];
            
            for (const item of items) {
                const titleElem = item.querySelector('.title a');
                const artistElem = item.querySelector('.artist, .meta a');
                const ratingElem = item.querySelector('.rating-star, .stars');
                const dateElem = item.querySelector('.date, .time');
                const commentElem = item.querySelector('.comment, .review');
                
                results.push({
                    title: titleElem?.textContent?.trim() || '',
                    url: titleElem?.href || '',
                    artist: artistElem?.textContent?.trim() || '',
                    rating: ratingElem?.className?.match(/rating(\d+)/)?.[1] || '',
                    date: dateElem?.textContent?.match(/\d{4}-\d{2}-\d{2}/)?.[0] || '',
                    comment: commentElem?.textContent?.trim() || ''
                });
            }
            
            return results;
        }, albumSelector.selector);
        
        console.log(`  提取 ${albums.length} 张专辑`);
        
        // 写入 CSV
        for (const album of albums) {
            const stars = album.rating ? '★'.repeat(parseInt(album.rating)) : '';
            const line = [
                csvEscape(album.title),
                csvEscape(album.artist),
                stars,
                '',  // 专辑评分（需要进入详情页）
                '',  // 发行年份（需要进入详情页）
                '',  // 厂牌（需要进入详情页）
                csvEscape(album.date),
                csvEscape(album.comment),
                csvEscape(album.url),
            ].join(',');
            
            fs.appendFileSync(CONFIG.outputFile, line + '\n', 'utf8');
            count++;
        }
        
        console.log(`  已导出 ${count} 张`);
        
        // 检查下一页
        const hasNext = await page.evaluate(() => {
            const next = document.querySelector('.paginator .next a, .next a, a.next');
            return next && next.style.display !== 'none';
        });
        
        if (!hasNext) {
            console.log('没有更多页面');
            break;
        }
        
        // 点击下一页
        await page.click('.paginator .next a, .next a, a.next');
        await page.waitForNetworkIdle({ timeout: 10000 });
        
        // 反爬虫延迟
        const delay = randomDelay(CONFIG.baseDelay, CONFIG.maxDelay);
        console.log(`  等待 ${delay/1000} 秒...`);
        await sleep(delay);
        
        pageNum++;
    }
    
    console.log('\n=== 导出完成 ===');
    console.log(`共导出 ${count} 张专辑`);
    console.log(`文件：${CONFIG.outputFile}`);
    
    await browser.disconnect();
}

main().catch(console.error);
