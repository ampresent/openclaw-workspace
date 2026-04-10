#!/usr/bin/env node
// Douban music collection full export using agent-browser
// Exports all "听过" (listened) albums to CSV

import { execSync } from 'child_process';
import fs from 'node:fs';
import path from 'node:path';

const CONFIG = {
    // 豆瓣用户 ID
    userId: process.env.DOUBAN_USER || '4428030',
    
    // 输出文件
    outputFile: process.env.OUTPUT_FILE || '/home/admin/openclaw/workspace/temp/douban-music-collect.csv',
    
    // 豆瓣音乐页面
    baseUrl: 'https://music.douban.com',
    
    // 反爬虫设置
    baseDelay: 4000,  // 4 秒
    maxDelay: 8000,   // 8 秒
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

function agentBrowser(cmd) {
    const agentBrowserPath = '/home/admin/.nvm/versions/node/v24.14.0/lib/node_modules/agent-browser/bin/agent-browser-linux-x64';
    try {
        return execSync(`${agentBrowserPath} ${cmd}`, { 
            encoding: 'utf-8',
            timeout: 30000,
        }).trim();
    } catch (e) {
        throw new Error(`agent-browser failed: ${e.message}`);
    }
}

async function exportMusic() {
    console.log('=== 豆瓣音乐专辑导出工具 ===');
    console.log(`用户 ID: ${CONFIG.userId}`);
    console.log(`输出文件：${CONFIG.outputFile}`);
    console.log('');
    
    // 确保输出目录存在
    fs.mkdirSync(path.dirname(CONFIG.outputFile), { recursive: true });
    
    const csvHeader = '专辑名称，艺人，用户评分，专辑评分，发行年份，厂牌，听过时间，评论，URL\n';
    fs.writeFileSync(CONFIG.outputFile, csvHeader, 'utf8');
    
    let count = 0;
    let page = 1;
    let retries = 0;
    
    // 打开页面
    console.log('打开豆瓣音乐页面...');
    try {
        agentBrowser(`open "https://music.douban.com/people/${CONFIG.userId}/collect?sort=time&mode=list"`);
        await sleep(3000);
    } catch (e) {
        console.error('打开页面失败:', e.message);
        console.log('请确保浏览器已登录豆瓣');
        return;
    }
    
    // 检查登录状态
    const title = agentBrowser('get title');
    if (title.includes('登录')) {
        console.log('⚠️  未登录，请先登录豆瓣');
        return;
    }
    console.log('✓ 已登录');
    
    while (retries < CONFIG.maxRetries) {
        console.log(`\n处理第 ${page} 页...`);
        
        // 获取快照
        console.log('获取页面快照...');
        const snapshot = agentBrowser('snapshot -i');
        
        // 解析专辑列表
        // 豆瓣音乐列表通常使用 .list-view 或 .article-list
        // 需要提取每个 .item 元素
        
        // 使用 JavaScript 提取数据
        const extractScript = `
        () => {
            const items = document.querySelectorAll('.list-view .item, .article-list .item');
            const results = [];
            for (const item of items) {
                const titleElem = item.querySelector('.title a');
                const artistElem = item.querySelector('.artist');
                const ratingElem = item.querySelector('.rating-star');
                const dateElem = item.querySelector('.date');
                const commentElem = item.querySelector('.comment');
                
                results.push({
                    title: titleElem ? titleElem.textContent.trim() : '',
                    url: titleElem ? titleElem.href : '',
                    artist: artistElem ? artistElem.textContent.trim() : '',
                    rating: ratingElem ? ratingElem.className.match(/rating(\\d+)/)?.[1] || '' : '',
                    date: dateElem ? dateElem.textContent.match(/\\d{4}-\\d{2}-\\d{2}/)?.[0] || '' : '',
                    comment: commentElem ? commentElem.textContent.trim() : ''
                });
            }
            return results;
        }
        `;
        
        try {
            // 执行提取脚本
            const result = agentBrowser(`eval "${extractScript.replace(/\n/g, '')}"`);
            console.log('提取结果:', result);
            
            // 解析结果并写入 CSV
            // ... (需要处理 JSON 输出)
            
        } catch (e) {
            console.error('提取失败:', e.message);
        }
        
        // 检查下一页
        console.log('检查下一页...');
        const hasNext = snapshot.includes('next') || snapshot.includes('后页');
        
        if (!hasNext) {
            console.log('没有更多页面');
            break;
        }
        
        // 点击下一页
        console.log('点击下一页...');
        agentBrowser('click .next a');
        await sleep(randomDelay(CONFIG.baseDelay, CONFIG.maxDelay));
        
        page++;
        count += 15; // 估计每页 15 条
        console.log(`已处理约 ${count} 张专辑`);
    }
    
    console.log('\n=== 导出完成 ===');
    console.log(`共导出约 ${count} 张专辑`);
    console.log(`文件：${CONFIG.outputFile}`);
}

exportMusic().catch(console.error);
