#!/usr/bin/env python3
"""
豆瓣音乐专辑导出工具 - 使用 Selenium 控制 Chrome（临时用户数据目录版本）
"""

import os
import sys
import time
import random
import csv
import json
import shutil
from datetime import datetime
from selenium import webdriver
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.chrome.options import Options
from webdriver_manager.chrome import ChromeDriverManager
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.common.exceptions import TimeoutException, WebDriverException

# 配置
CONFIG = {
    'base_url': 'https://music.douban.com/mine?status=collect',
    
    # 临时用户数据目录
    'user_data_dir': os.path.expanduser('~/.config/google-chrome-douban-export-temp'),
    
    'output_file': 'temp/douban-albums.csv',
    
    'base_delay': 4,
    'delay_jitter': 1,
    'max_retries': 5,
    'retry_delay': 5,
    'max_retry_delay': 60,
    
    'fail_keywords': ['访问太频繁', '验证', 'captcha', '403', '登录豆瓣'],
}

def random_delay(base, jitter):
    delay = base + random.uniform(-jitter, jitter)
    time.sleep(delay)

def is_blocked(driver):
    try:
        title = driver.title.lower()
        url = driver.current_url.lower()
        for keyword in CONFIG['fail_keywords']:
            if keyword.lower() in title or keyword.lower() in url:
                return True
        return False
    except:
        return False

def create_driver():
    """创建 Chrome WebDriver（使用临时用户数据目录）"""
    options = Options()
    
    # 使用临时用户数据目录
    options.add_argument(f'--user-data-dir={CONFIG["user_data_dir"]}')
    options.add_argument('--no-sandbox')
    options.add_argument('--disable-dev-shm-usage')
    options.add_argument('--disable-gpu')
    options.add_argument('--window-size=1280,800')
    
    # 不自动关闭
    options.add_experimental_option('detach', True)
    
    # 使用 webdriver-manager 自动下载匹配的 ChromeDriver
    try:
        service = Service(ChromeDriverManager().install())
        driver = webdriver.Chrome(service=service, options=options)
        return driver
    except WebDriverException as e:
        print(f"启动 Chrome 失败：{e}")
        return None

def wait_for_login(driver, timeout=300):
    """等待用户登录"""
    print('')
    print('=' * 60)
    print('请登录豆瓣')
    print('=' * 60)
    print('')
    print(f'1. 浏览器已打开，请访问：{CONFIG["base_url"]}')
    print('')
    print('2. 完成豆瓣登录（手机号/验证码或其他方式）')
    print('')
    print('3. 登录后，页面会自动跳转到"我听过的专辑"')
    print('')
    print(f'4. 等待 {timeout} 秒后自动开始导出...')
    print('')
    
    start_time = time.time()
    last_title = ''
    
    while time.time() - start_time < timeout:
        try:
            title = driver.title
            url = driver.current_url
            
            # 检查是否已登录（不在登录页面）
            if '登录' not in title and 'passport' not in url.lower():
                print(f'✓ 检测到登录状态！页面：{title}')
                random_delay(2, 0.5)
                return True
            
            if title != last_title:
                print(f'当前页面：{title}')
                last_title = title
            
            time.sleep(2)
            
        except Exception as e:
            time.sleep(2)
    
    print('⚠️  等待登录超时')
    return False

def export_albums():
    """导出专辑数据"""
    print('=== 豆瓣音乐专辑导出工具 ===')
    print(f'时间：{datetime.now().strftime("%Y-%m-%d %H:%M:%S")}')
    print(f'临时用户数据目录：{CONFIG["user_data_dir"]}')
    print('')
    
    # 确保输出目录存在
    os.makedirs(os.path.dirname(CONFIG['output_file']), exist_ok=True)
    
    # 创建驱动
    print('启动 Chrome（新窗口）...')
    driver = create_driver()
    
    if not driver:
        print('')
        print('启动失败。请关闭所有 Chrome 窗口后重试。')
        return False
    
    try:
        # 打开页面
        print(f'打开豆瓣音乐页面...')
        driver.get(CONFIG['base_url'])
        
        # 等待登录
        if not wait_for_login(driver, timeout=300):
            print('')
            print('登录超时。请关闭浏览器，然后重新运行脚本。')
            return False
        
        print('')
        
        # 截图
        screenshot_path = 'temp/douban-logged-in.png'
        driver.save_screenshot(screenshot_path)
        print(f'截图已保存到：{screenshot_path}')
        print('')
        
        # 等待专辑列表加载
        print('分析页面结构...')
        
        selectors = [
            ('.article-list li', '文章列表'),
            ('.list li', '列表'),
            ('li.item', '项目'),
        ]
        
        album_selector = None
        for selector, name in selectors:
            try:
                elements = driver.find_elements(By.CSS_SELECTOR, selector)
                if len(elements) > 0:
                    album_selector = selector
                    print(f'找到专辑列表选择器：{selector} ({name}, {len(elements)} 个元素)')
                    break
            except:
                continue
        
        if not album_selector:
            print('未找到标准专辑列表选择器')
            print('页面源代码（前 2000 字符）:')
            print(driver.page_source[:2000])
            return False
        
        albums = driver.find_elements(By.CSS_SELECTOR, album_selector)
        print(f'第一页找到 {len(albums)} 张专辑')
        
        # 检查分页
        try:
            next_btn = driver.find_element(By.CSS_SELECTOR, 'link[rel="next"], .next a, a.next')
            has_next = next_btn.is_displayed() if next_btn else False
        except:
            has_next = False
        print(f'有分页：{has_next}')
        
        print('')
        print('✓ 页面结构分析完成')
        print('')
        
        # 开始导出
        print('开始导出专辑数据...')
        print(f'输出文件：{CONFIG["output_file"]}')
        print('')
        
        fieldnames = ['专辑名称', '艺人', '用户评分', '专辑评分', '发行年份', '厂牌', '听过时间', '评论', 'URL']
        
        count = 0
        page = 1
        max_pages = 250
        
        with open(CONFIG['output_file'], 'w', newline='', encoding='utf-8-sig') as csvfile:
            writer = csv.DictWriter(csvfile, fieldnames=fieldnames)
            writer.writeheader()
            
            while page <= max_pages:
                print(f'处理第 {page} 页...')
                
                if is_blocked(driver):
                    print('⚠️  检测到访问限制')
                    break
                
                albums = driver.find_elements(By.CSS_SELECTOR, album_selector)
                
                for album in albums:
                    try:
                        data = extract_album_info(album, driver)
                        writer.writerow(data)
                        count += 1
                    except Exception as e:
                        print(f'  提取专辑失败：{e}')
                        continue
                
                print(f'  已导出 {count} 张专辑')
                
                # 检查下一页
                try:
                    next_btn = driver.find_element(By.CSS_SELECTOR, '.next a, a.next, link[rel="next"]')
                    if not next_btn.is_displayed():
                        print('没有更多页面')
                        break
                    
                    next_btn.click()
                    random_delay(2, 0.5)
                    
                    WebDriverWait(driver, 10).until(
                        lambda d: len(d.find_elements(By.CSS_SELECTOR, album_selector)) > 0
                    )
                    
                except TimeoutException:
                    print('没有更多页面')
                    break
                except Exception as e:
                    print(f'翻页失败：{e}')
                    break
                
                # 反爬虫延迟
                delay = CONFIG['base_delay'] + random.uniform(-CONFIG['delay_jitter'], CONFIG['delay_jitter'])
                print(f'  等待 {delay:.1f} 秒（反爬虫）...')
                time.sleep(delay)
                
                page += 1
        
        print('')
        print(f'✓ 导出完成！')
        print(f'共导出 {count} 张专辑')
        print(f'文件：{CONFIG["output_file"]}')
        
        return True
        
    except Exception as e:
        print(f'错误：{e}')
        import traceback
        traceback.print_exc()
        return False
    
    finally:
        print('')
        print('浏览器保持打开状态，10 秒后自动关闭...')
        time.sleep(10)
        try:
            driver.quit()
        except:
            pass
        
        # 清理临时目录
        print('清理临时文件...')
        try:
            shutil.rmtree(CONFIG['user_data_dir'], ignore_errors=True)
            print(f'已删除：{CONFIG["user_data_dir"]}')
        except Exception as e:
            print(f'清理失败：{e}')

def extract_album_info(album_element, driver):
    """提取单个专辑信息"""
    data = {
        '专辑名称': '',
        '艺人': '',
        '用户评分': '',
        '专辑评分': '',
        '发行年份': '',
        '厂牌': '',
        '听过时间': '',
        '评论': '',
        'URL': '',
    }
    
    try:
        # 专辑名称和 URL
        title_elem = album_element.find_element(By.CSS_SELECTOR, 'a.title, a')
        data['专辑名称'] = title_elem.text.strip()
        data['URL'] = title_elem.get_attribute('href')
        
        # 艺人
        try:
            artist_elem = album_element.find_element(By.CSS_SELECTOR, '.artist, .meta a')
            data['艺人'] = artist_elem.text.strip()
        except:
            pass
        
        # 评分
        try:
            rating_elem = album_element.find_element(By.CSS_SELECTOR, '.rating, .stars')
            data['用户评分'] = rating_elem.text.strip()
        except:
            pass
        
        # 听过时间
        try:
            time_elem = album_element.find_element(By.CSS_SELECTOR, '.time, .date')
            data['听过时间'] = time_elem.text.strip()
        except:
            pass
        
        # 评论
        try:
            comment_elem = album_element.find_element(By.CSS_SELECTOR, '.comment, .review')
            data['评论'] = comment_elem.text.strip()
        except:
            pass
        
    except Exception as e:
        print(f'    提取失败：{e}')
    
    return data

if __name__ == '__main__':
    success = export_albums()
    sys.exit(0 if success else 1)
