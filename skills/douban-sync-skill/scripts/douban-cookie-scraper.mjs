#!/usr/bin/env node
// Douban full export with cookies - outputs CSV
// Uses provided cookies for authentication

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import http from 'node:http';
import https from 'node:https';

const USER = process.env.DOUBAN_USER || '4428030';
const BASE_DIR = process.env.DOUBAN_OUTPUT_DIR || path.join(os.homedir(), 'douban-sync');
const OUTPUT_DIR = path.join(BASE_DIR, USER);

// Cookies from user
const COOKIES = [
  'll="108288"',
  'bid=vo4bhsnvkKg',
  'ck=_eat',
  'ap_v=0,6.0',
  'push_noty_num=0',
  'push_doumail_num=0',
  '__utmv=30149280.442',
  '_pk_id.100001.afe6=c4b2cf0a29a31c2c.1774209883.',
].join('; ');

const CSV_HEADER = 'title,url,date,rating,status,comment\n';

function sleep(ms) { return new Promise(r => setTimeout(r, ms)); }

function csvEscape(str) {
  if (!str) return '';
  if (str.includes(',') || str.includes('"') || str.includes('\n')) {
    return '"' + str.replace(/"/g, '""') + '"';
  }
  return str;
}

function ratingStars(rating) {
  if (!rating || rating === 0) return '';
  return '★'.repeat(rating);
}

function parseListPage(html) {
  const items = [];
  const itemBlocks = html.split(/<div class="item">/);

  for (let i = 1; i < itemBlocks.length; i++) {
    const block = itemBlocks[i];

    const titleMatch = block.match(/<a[^>]*href="(https:\/\/(book|movie|music)\.douban\.com\/subject\/\d+\/)"[^>]*>([\s\S]*?)<\/a>/);
    if (!titleMatch) continue;
    const link = titleMatch[1];
    const title = titleMatch[2].replace(/<[^>]+>/g, '').trim();

    const dateMatch = block.match(/<span class="date">([\s\S]*?)<\/span>/);
    let date = '', rating = 0;
    if (dateMatch) {
      const dm = dateMatch[1].match(/(\d{4}-\d{2}-\d{2})/);
      if (dm) date = dm[1];
      const rm = dateMatch[1].match(/rating(\d+)-t/);
      if (rm) rating = parseInt(rm[1]);
    }

    const commentMatch = block.match(/<span class="comment">([\s\S]*?)<\/span>/);
    const comment = commentMatch ? commentMatch[1].replace(/<[^>]+>/g, '').trim() : '';

    items.push({ title, link, date, rating, comment });
  }
  return items;
}

function parseGamePage(html) {
  const items = [];
  const itemBlocks = html.split(/<div class="common-item">/);

  for (let i = 1; i < itemBlocks.length; i++) {
    const block = itemBlocks[i];

    const titleMatch = block.match(/<div class="title">\s*<a href="(https:\/\/www\.douban\.com\/game\/\d+\/)">([\s\S]*?)<\/a>/);
    if (!titleMatch) continue;
    const link = titleMatch[1];
    const title = titleMatch[2].replace(/<[^>]+>/g, '').trim();

    let rating = 0;
    const ratingMatch = block.match(/allstar(\d+)/);
    if (ratingMatch) rating = parseInt(ratingMatch[1]) / 10;

    let date = '';
    const dateMatch = block.match(/<span class="date">([\s\S]*?)<\/span>/);
    if (dateMatch) {
      const dm = dateMatch[1].match(/(\d{4}-\d{2}-\d{2})/);
      if (dm) date = dm[1];
    }

    items.push({ title, link, date, rating, comment: '' });
  }
  return items;
}

function fetchPage(url) {
  return new Promise((resolve, reject) => {
    const mod = url.startsWith('https') ? https : http;
    const options = {
      headers: {
        'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
        'Accept': 'text/html,application/xhtml+xml',
        'Cookie': COOKIES,
        'Referer': 'https://www.douban.com/',
      }
    };
    
    mod.get(url, options, res => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        if (res.statusCode >= 400) {
          reject(new Error(`HTTP ${res.statusCode}`));
        } else {
          resolve(data);
        }
      });
    }).on('error', reject);
  });
}

async function fetchAllItems(base, userPath, type) {
  const allItems = [];
  let start = 0;
  let retries = 0;
  const MAX_RETRIES = 5;
  const isGame = type === 'game';
  const pageSize = isGame ? 15 : 30;

  while (true) {
    const sep = userPath.includes('?') ? '&' : '?';
    const url = `${base}/people/${USER}/${userPath}?start=${start}&sort=time&rating=all&filter=all&mode=list`;
    console.log(`Fetching: ${url}`);

    try {
      const html = await fetchPage(url);
      
      // Check if response is login page
      if (html.includes('登录') || html.includes('passport')) {
        console.log('  Response is login page - cookies may be invalid');
        break;
      }
      
      const items = isGame ? parseGamePage(html) : parseListPage(html);

      if (items.length === 0) { console.log('  No items, stopping.'); break; }
      console.log(`  Found ${items.length} items`);
      allItems.push(...items);
      retries = 0;

      if (items.length < pageSize) { console.log(`  Last page.`); break; }
      start += pageSize;
      
      // Rate limiting
      await sleep(3000);
    } catch (err) {
      console.error(`  Error: ${err.message}`);
      if ((err.message.includes('403') || err.message.includes('418')) && retries < MAX_RETRIES) {
        retries++;
        const delay = 10000 * Math.pow(2, retries - 1);
        console.log(`  Rate limited, retry ${retries}/${MAX_RETRIES}, waiting ${delay/1000}s...`);
        await sleep(delay);
        continue;
      }
      break;
    }
  }
  return allItems;
}

function itemToCsvLine(item, status) {
  return [
    csvEscape(item.title),
    csvEscape(item.link),
    csvEscape(item.date),
    csvEscape(ratingStars(item.rating)),
    csvEscape(status),
    csvEscape(item.comment),
  ].join(',');
}

async function main() {
  console.log('=== Douban Music Export (with cookies) ===');
  console.log(`User: ${USER}`);
 console.log(`Output: ${OUTPUT_DIR}`);
  console.log('');
  
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });

  // Only export music (听过)
  const musicCategory = { base: 'https://music.douban.com', type: 'music', path: 'collect', status: '听过', file: '音乐.csv' };
  
  console.log(`\n=== ${musicCategory.status} (${musicCategory.type}) ===`);
  const items = await fetchAllItems(musicCategory.base, musicCategory.path, musicCategory.type);
  console.log(`Total: ${items.length} items`);

  const filePath = path.join(OUTPUT_DIR, musicCategory.file);
  const lines = items.map(item => itemToCsvLine(item, musicCategory.status));
  fs.writeFileSync(filePath, CSV_HEADER + lines.join('\n') + '\n', 'utf8');
  console.log(`Written ${lines.length} rows to ${filePath}`);

  console.log('\nDone!');
}

main().catch(console.error);
