/*
 * legacy_network.c — 某嵌入式项目的网络模块
 *
 * 编译失败场景：
 *   GCC 14+ 默认将 -Wimplicit-function-declaration 升级为 -Werror
 *   本文件遗漏了 <string.h> 头文件，导致 memcpy() 隐式声明
 *
 * 编译命令：
 *   gcc -O2 -Wall -Werror=implicit-function-declaration -c legacy_network.c
 *
 * 错误输出：
 *   legacy_network.c:42:5: error: implicit declaration of function 'memcpy'
 *       [-Wimplicit-function-declaration]
 *      42 |     memcpy(dst, src, len);
 *         |     ^~~~~~
 */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
// #include <string.h>  /* ← 这行被注释掉了，触发编译错误 */

#define MAX_PACKET_SIZE 1500
#define HEADER_MAGIC    0xEVO

typedef struct {
    uint32_t magic;
    uint16_t length;
    uint8_t  type;
    uint8_t  payload[MAX_PACKET_SIZE];
} network_packet_t;

typedef struct {
    uint32_t packets_received;
    uint32_t packets_dropped;
    uint32_t bytes_total;
} network_stats_t;

static network_stats_t stats = {0};

/* 初始化网络模块 */
void net_init(void) {
    stats.packets_received = 0;
    stats.packets_dropped = 0;
    stats.bytes_total = 0;
    printf("[net] initialized, max packet size: %d\n", MAX_PACKET_SIZE);
}

/* 解析数据包 — 这里触发隐式声明错误 */
int net_parse_packet(const uint8_t *raw, size_t raw_len, network_packet_t *pkt) {
    if (raw_len < 8) {
        stats.packets_dropped++;
        return -1;
    }

    /* ⚡ 触发点：memcpy 需要 #include <string.h> */
    memcpy(pkt, raw, raw_len < sizeof(network_packet_t) ? raw_len : sizeof(network_packet_t));

    if (pkt->magic != HEADER_MAGIC) {
        stats.packets_dropped++;
        return -2;
    }

    stats.packets_received++;
    stats.bytes_total += pkt->length;
    return 0;
}

/* 构造回复包 — 同样用了 memcpy */
int net_build_reply(uint8_t type, const uint8_t *data, size_t data_len,
                    network_packet_t *reply) {
    reply->magic = HEADER_MAGIC;
    reply->type = type;
    reply->length = (uint16_t)data_len;

    /* ⚡ 触发点 */
    if (data_len > 0 && data_len <= MAX_PACKET_SIZE) {
        memcpy(reply->payload, data, data_len);
    }

    return 0;
}

/* 打印统计信息 */
void net_print_stats(void) {
    printf("[net] received: %u, dropped: %u, bytes: %u\n",
           stats.packets_received, stats.packets_dropped, stats.bytes_total);
}
