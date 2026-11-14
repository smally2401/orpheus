#include "tags.h"

#include "util.h"

#include <glib.h>
#include <glib/gstdio.h>

#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_TAG_BYTES (64u * 1024u * 1024u)

typedef struct
{   
    TagFn fn;
    void* ud;
    bool want_picture;
    unsigned char* pic;
    size_t pic_len;
    int pic_type;
    bool got_text;
} Ctx;

static uint32_t be32(const unsigned char* p)
{
    return (uint32_t)p[0] << 24
        | (uint32_t)p[1] << 16
        | (uint32_t)p[2] << 8
        | p[3];
}

static uint32_t be24(const unsigned char* p)
{
    return (uint32_t)p[0] << 16
        | (uint32_t)p[1] << 8
        | p[2];
}


static uint32_t le32(const unsigned char* p)
{
    return (uint32_t)p[3] << 24
        | (uint32_t)p[2] << 16
        | (uint32_t)p[1] << 8
        | p[0];
}

static uint32_t le16(const unsigned char* p)
{
    return (uint32_t)p[1] << 8
        | p[0];
}

static uint64_t le64(const unsigned char* p)
{
    return (uint64_t)le32(p + 4) << 32
        | le32(p);
}

static uint32_t syncsafe(const unsigned char* p)
{
    return (uint32_t)(p[0] & 0x7f) << 21
        | (uint32_t)(p[1] & 0x7f) << 14
        | (uint32_t)(p[2] & 0x7f) << 7
        | (p[3] & 0x7f);
}

static bool read_at(FILE* f, long off, void* buf, size_t n)
{
    if (fseek(f, off, off < 0 ? SEEK_END : SEEK_SET) != 0)
    {
        return false;
    }

    return fread(buf, 1, n, f) == n;
}

static unsigned char* read_alloc(FILE* f, long off, size_t n)
{
    if (n > MAX_TAG_BYTES)
    {
        return NULL;
    }

    unsigned char* b = xmalloc(n);
    if (!read_at(f, off, b, n))
    {
        free(b);
        return NULL;
    }

    return b;
}

static long file_size(FILE* f)
{
    if (fseek(f, 0, SEEK_END) != 0)
    {
        return -1;
    }

    return ftell(f);
}

static void emit(Ctx* c, const char* key, const char* val)
{
    if (!val || !*val)
    {
        return;
    }

    c->got_text = true;
    if (c->fn)
    {
        c->fn(key, val, c->ud);
    }
}

static void emit_duration(Ctx* c, double secs)
{
    if (secs <= 0 || !c->fn)
    {
        return;
    }

    char buf[64];
    snprintf(buf, sizeof buf, "%.3f", secs);
    c->fn("duration", buf, c->ud);
}

static void offer_picture(Ctx* c, int type, const unsigned char* data, size_t len)
{
    if (!c->want_picture || len == 0)
    {
        return;
    }

    if (c->pic && (c->pic_type == 3 || type != 3))
    {
        return;
    }

    free(c->pic);
    c->pic = xmalloc(len);
    memcpy(c->pic, data, len);
    c->pic_len = len;
    c->pic_type = type;
}

static char* to_utf8(const unsigned char* s, size_t n, int enc)
{
    const char* from =
        enc == 0 ? "ISO-8859-1" :
        enc == 1 ? "UTF-16" :
        enc == 2 ? "UTF-16BE" :
        NULL;

    gsize len = n;
    char* raw;
    if (from)
    {
        raw = g_convert((const char*)s, (gssize)n, "UTF-8", from, NULL, &len, NULL);
        if (!raw)
        {
            return NULL;
        }
    }
    else
    {
        raw = g_malloc(n + 1);
        memcpy(raw, s, n);
    }

    while (len > 0 && raw[len - 1] == '\0')
    {
        len--;
    }

    for (gsize i = 0; i < len; i++)
    {
        if (raw[i] == '\0')
        {
            raw[i] = '\x01';
        }
    }

    char* valid = g_utf8_make_valid(raw, (gssize)len);
    g_free(raw);
    GString* out = g_string_sized_new(len + 8);
    for (const unsigned char* p = (const unsigned char*)valid; *p; p++)
    {
        if (*p == 1)
        {
            g_string_append(out, "; ");
        }
        else if (p[0] == 0xEF && p[1] == 0xBB && p[2] == 0xBF)
        {
            p += 2;
        }
        else
        {
            g_string_append_c(out, (char)*p);
        }
    }

    g_free(valid);
    return g_strstrip(g_string_free(out, FALSE));
}

static size_t enc_strlen(const unsigned char* s, size_t n, int enc, size_t* skip)
{
    size_t i;
    if (enc == 1 || enc == 2)
    {
        for (i = 0; i + 1 < n; i += 2)
        {
            if (s[i] == 0 && s[i + 1] == 0) {
                *skip = i + 2;
                return i;
            }
        }
    }
    else
    {
        for (i = 0; i < n; i++)
        {
            if (s[i] == 0)
            {
                *skip = i + 1;
                return i;
            }
        }
    }

    *skip = n;
    return n;
}

static const char* id3_key(const char* id)
{
    static const char* map[][2] = {
        { "TIT2", "title" },
        { "TPE1", "artist" },
        { "TALB", "album" },
        { "TPE2", "albumartist" },
        { "TRCK", "tracknumber" },
        { "TPOS", "discnumber" },
        { "TDRC", "date" },
        { "TYER", "date" },
        { "TCON", "genre" },
        { "TCOM", "composer" },
        { "TIT1", "grouping" },
        { "TBPM", "bpm" },
        { "TPUB", "label" },
        { "TIT3", "subtitle" },
        { "TT2", "title" },
        { "TP1", "artist" },
        { "TAL", "album" },
        { "TP2", "albumartist" },
        { "TRK", "tracknumber" },
        { "TPA", "discnumber" },
        { "TYE", "date" },
        { "TCO", "genre" },
        { "TCM", "composer" },
    };

    for (size_t i = 0; i < sizeof map / sizeof map[0]; i++)
    {
        if (!strcmp(map[i][0], id))
        {
            return map[i][1];
        }
    }

    return NULL;
}

static size_t unsync(unsigned char* b, size_t n)
{
    size_t o = 0;
    for (size_t i = 0; i < n; i++)
    {
        b[o] = b[i];
        o++;
        if (b[i] == 0xFF && i + 1 < n && b[i + 1] == 0x00)
        {
            i++;
        }
    }

    return o;
}

static void id3_frame(Ctx* c, const char* id, unsigned char* d, size_t n, int ver)
{
    if (n < 1)
    {
        return;
    }

    int enc = d[0];
    if (enc > 3)
    {
        return;
    }

    const char* key = id3_key(id);
    if (key)
    {
        char* v = to_utf8(d + 1, n - 1, enc);
        if (v)
        {
            emit(c, key, v);
            g_free(v);
        }
    }
    else if (!strcmp(id, "TXXX") || !strcmp(id, "TXX"))
    {
        size_t skip;
        size_t dl = enc_strlen(d + 1, n - 1, enc, &skip);
        char* desc = to_utf8(d + 1, dl, enc);
        char* val = to_utf8(d + 1 + skip, n - 1 - skip, enc);
        if (desc && val)
        {
            char* k = g_ascii_strdown(desc, -1);
            emit(c, k, val);
            g_free(k);
        }

        g_free(desc);
        g_free(val);
    }
    else if (!strcmp(id, "COMM") || !strcmp(id, "COM"))
    {
        if (n < 4)
        {
            return;
        }

        size_t skip;
        enc_strlen(d + 4, n - 4, enc, &skip);
        char* val = to_utf8(d + 4 + skip, n - 4 - skip, enc);
        if (val)
        {
            emit(c, "comment", val);
        }

        g_free(val);
    }
    else if (c->want_picture && (!strcmp(id, "APIC") || !strcmp(id, "PIC")))
    {
        size_t p = 1;
        if (ver == 2)
        {
            p += 3;
        }
        else
        {
            while (p < n && d[p])
            {
                p++;
            }

            p++;
        }

        if (p >= n)
        {
            return;
        }

        int type = d[p];
        p++;
        size_t skip;
        enc_strlen(d + p, n - p, enc, &skip);
        p += skip;
        if (p < n)
        {
            offer_picture(c, type, d + p, n - p);
        }
    }
}

static void parse_id3v2(Ctx* c, unsigned char* buf, size_t len)
{
    if (len < 10 || memcmp(buf, "ID3", 3) != 0)
    {
        return;
    }

    int ver = buf[3];
    int flags = buf[5];
    if (ver < 2 || ver > 4)
    {
        return;
    }

    unsigned char* p = buf + 10;
    size_t n = len - 10;
    if ((flags & 0x80) && ver < 4)
    {
        n = unsync(p, n);
    }

    if (flags & 0x40)
    {
        if (ver == 3 && n >= 4)
        {
            size_t ext = be32(p) + 4;
            if (ext > n)
            {
                return;
            }

            p += ext;
            n -= ext;
        }
        else if (ver == 4 && n >= 4)
        {
            size_t ext = syncsafe(p);
            if (ext > n)
            {
                return;
            }

            p += ext;
            n -= ext;
        }
    }

    size_t hdr = ver == 2 ? 6 : 10;
    while (n >= hdr && p[0])
    {
        char id[5] = { 0 };
        size_t fsz;
        int fflags = 0;
        if (ver == 2)
        {
            memcpy(id, p, 3);
            fsz = be24(p + 3);
        }
        else
        {
            memcpy(id, p, 4);
            fsz = ver == 4 ? syncsafe(p + 4) : be32(p + 4);
            fflags = p[8] << 8 | p[9];
        }

        if (fsz > n - hdr)
        {
            break;
        }

        unsigned char* d = p + hdr;
        size_t dn = fsz;
        bool skip = false;
        if (ver == 3 && (fflags & 0x00C0))
        {
            skip = true;
        }

        if (ver == 4)
        {
            if (fflags & 0x000C)
            {
                skip = true;
            }

            if ((fflags & 0x0001) && dn >= 4)
            {
                d += 4;
                dn -= 4;
            }

            if (fflags & 0x0002)
            {
                dn = unsync(d, dn);
            }
        }

        if (!skip)
        {
            id3_frame(c, id, d, dn, ver);
        }

        p += hdr + fsz;
        n -= hdr + fsz;
    }
}

static size_t id3v2_size_at(FILE* f, long off)
{
    unsigned char h[10];
    if (!read_at(f, off, h, 10) || memcmp(h, "ID3", 3) != 0)
    {
        return 0;
    }

    return 10 + syncsafe(h + 6) + ((h[5] & 0x10) ? 10 : 0);
}

static void read_id3v1(Ctx* c, FILE* f)
{
    unsigned char t[128];
    if (!read_at(f, -128, t, 128) || memcmp(t, "TAG", 3) != 0)
    {
        return;
    }

    struct {
        const char* key;
        int off;
        int len;
    } fields[] = {
        { "title", 3, 30 },
        { "artist", 33, 30 },
        { "album", 63, 30 },
        { "date", 93 , 4},
    };

    for (size_t i = 0; i < 4; i++)
    {
        size_t l = 0;
        while (l < (size_t)fields[i].len && t[fields[i].off + l])
        {
            l++;
        }

        char* v = to_utf8(t + fields[i].off, l, 0);
        if (v)
        {
            emit(c, fields[i].key, v);
        }

        g_free(v);
    }

    if (t[125] == 0 && t[126] != 0)
    {
        char num[8];
        snprintf(num, sizeof num, "%d", t[126]);
        emit(c, "tracknumber", num);
    }
}

static double mp3_duration(FILE* f, long start, long fsize)
{

    static const int br_tab[2][3][15] = {
        { { 0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448 },
          { 0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384 },
          { 0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320 } },
        { { 0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256 },
          { 0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160 },
          { 0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160 } },
    };

    static const int sr_tab[3] = { 44100, 48000, 32000 };
    enum { WIN = 65536 };
    unsigned char* b = xmalloc(WIN);
    if (fseek(f, start, SEEK_SET) != 0)
    {
        free(b);
        return 0;
    }

    size_t n = fread(b, 1, WIN, f);
    double result = 0;
    for (size_t i = 0; i + 4 <= n; i++)
    {
        if (b[i] != 0xFF || (b[i + 1] & 0xE0) != 0xE0)
        {
            continue;
        }

        int v = (b[i + 1] >> 3) & 3;
        int layer = (b[i + 1] >> 1) & 3;
        int bri = b[i + 2] >> 4;
        int sri = (b[i + 2] >> 2) & 3;
        int pad = (b[i + 2] >> 1) & 1;
        int mono = (b[i + 3] >> 6) == 3;
        if (v == 1 || layer == 0 || bri == 0 || bri == 15 || sri == 3)
        {
            continue;
        }

        int mpeg1 = v == 3;
        int li = 3 - layer;
        int br = br_tab[mpeg1 ? 0 : 1][li][bri] * 1000;
        int sr = sr_tab[sri] >> (v == 3 ? 0 : v == 2 ? 1 : 2);
        int spf = li == 0 ? 382 : (li == 2 && !mpeg1) ? 576 : 1152;
        size_t flen = li == 0 ?
            (size_t)(12 * br / sr + pad) * 4 :
            (size_t)(spf / 8 * br / sr + pad);

        if (i + flen + 2 <= n && (b[i + flen] != 0xFF 
            || (b[i + flen + 1] & 0xE0) != 0xE0))
        {
            continue;
        }

        size_t xo = i + (mpeg1 ? (mono ? 21 : 36) : (mono ? 13 : 21));
        if (xo + 12 <= n &&
            (!memcmp(b + xo, "Xing", 4) || !memcmp(b + xo, "Info", 4))) 
        {
            if (be32(b + xo + 4) & 1)
            {
                result = (double)be32(b + xo + 8) * spf / sr;
                break;
            }
        }

        if (i + 36 + 18 <= n && !memcmp(b + i + 36, "VBRI", 4))
        {
            result = (double)be32(b + i + 36 + 14) * spf / sr;
            break;
        }

        long audio = fsize - start - (long)i;
        unsigned char t[3];
        if (read_at(f, -128, t, 3) && !memcmp(t, "TAG", 3))
        {
            audio -= 128;
        }

        result = audio * 8.0 / br;
        break;
    }

    free(b);
    return result;
}

static void parse_flac_picture(Ctx* c, const unsigned char* b, size_t n)
{
    if (n < 8)
    {
        return;
    }

    int type = (int)be32(b);
    size_t p = 4;
    size_t l = be32(b + p);
    p += 4;
    if (l > n - p || n - p - l < 4)
    {
        return;
    }

    p += l;
    l = be32(b + p);
    p += 4;
    if (l > n - p || n - p - l < 20)
    {
        return;
    }

    p += l + 16;
    l = be32(b + p);
    p += 4;
    if (l > n - p)
    {
        return;
    }

    offer_picture(c, type, b + p, l);
}

static void parse_vorbis_comments(Ctx* c, const unsigned char* b, size_t n)
{
    if (n < 8)
    {
        return;
    }

    size_t p = 4 + le32(b);
    if (p > n - 4)
    {
        return;
    }

    uint32_t count = le32(b + p);
    p += 4;
    for (uint32_t i = 0; i < count && p + 4 <= n; i++)
    {
        size_t l = le32(b + p);
        p += 4;
        if (l > n - p)
        {
            break;
        }

        const char* s = (const char*)b + p;
        const char* eq = memchr(s, '=', l);
        if (!eq)
        {
            p += l;
            continue;
        }

        char* key = g_ascii_strdown(s, eq - s);
        if (!strcmp(key, "metadata_block_picture"))
        {
            if (c->want_picture)
            {
                char* b64 = g_strndup(eq + 1, l - (size_t)(eq + 1 - s));
                gsize dl = 0;
                guchar* d = g_base64_decode(b64, &dl);
                parse_flac_picture(c, d, dl);
                g_free(d);
                g_free(b64);
            }
        }
        else
        {
            char* val = g_utf8_make_valid(eq + 1, (gssize)(l - (size_t)(eq + 1 - s)));
            emit(c, key, val);
            g_free(val);
        }

        g_free(key);
        p += l;
    }
}

static void read_flac(Ctx* c, FILE* f, long off)
{
    unsigned char h[4];
    off += 4;
    for (int last = 0; !last;)
    {
        if (!read_at(f, off, h, 4))
        {
            return;
        }

        last = h[0] & 0x80;
        int type = h[0] & 0x7f;
        size_t len = be24(h + 1);
        off += 4;
        if (type != 0 && type != 4 && !(type == 6 && c->want_picture))
        {
            off += (long)len;
            continue;
        }

        unsigned char* b = read_alloc(f, off, len);
        if (!b)
        {
            return;
        }

        if (type == 0 && len >= 18)
        {
            uint32_t rate = be32(b + 10) >> 12;
            if (rate)
            {
                uint64_t total = ((uint64_t)(b[13] & 0x0f) << 32) | be32(b + 14);
                emit_duration(c, (double)total / rate);
            }
        }
        else if (type == 4)
        {
            parse_vorbis_comments(c, b, len);
        }
        else if (type == 6)
        {
            parse_flac_picture(c, b, len);
        }

        free(b);
        off += (long)len;
    }
}

static void read_ogg(Ctx* c, FILE* f)
{
    GByteArray* pk[2] = {
        g_byte_array_new(),
        g_byte_array_new(),
    };

    int cur = 0;
    long off = 0;
    unsigned char h[27];
    unsigned char seg[255];
    while (cur < 2)
    {
        if (!read_at(f, off, h, 27) || memcmp(h, "OggS", 4) != 0)
        {
            break;
        }

        int nseg = h[26];
        if (fread(seg, 1, (size_t)nseg, f) != (size_t)nseg)
        {
            break;
        }

        size_t body = 0;
        for (int i = 0; i < nseg; i++)
        {
            body += seg[i];
        }

        unsigned char* b = read_alloc(f, off + 27 + nseg, body);
        if (!b)
        {
            break;
        }

        size_t p = 0;
        for (int i = 0; i < nseg && cur < 2; i++)
        {
            g_byte_array_append(pk[cur], b + p, seg[i]);
            p += seg[i];
            if (seg[i] < 255)
            {
                cur++;
            }

            if (pk[cur < 2 ? cur : 1]->len > MAX_TAG_BYTES)
            {
                cur = 3;
            }
        }

        free(b);
        off += 27 + nseg + (long)body;
    }

    const unsigned char* id = pk[0]->data;
    const unsigned char* cm = pk[1]->data;
    size_t idn = pk[0]->len;
    size_t cmn = pk[1]->len;
    double rate = 0;
    uint64_t preskip = 0;
    if (idn >= 16 && !memcmp(id, "\x01vorbis", 7))
    {
        rate = le32(id + 12);
        if (cmn > 7 && !memcmp(cm, "\x03vorbis", 7))
        {
            parse_vorbis_comments(c, cm + 7, cmn - 7);
        }
    }
    else if (idn >= 12 && !memcmp(id, "OpusHead", 8))
    {
        rate = 48000;
        preskip = le16(id + 10);
        if (cmn > 8 && !memcmp(cm, "OpusTags", 8))
        {
            parse_vorbis_comments(c, cm + 8, cmn - 8);
        }
    }

    g_byte_array_unref(pk[0]);
    g_byte_array_unref(pk[1]);
    long fsz = file_size(f);
    if (rate <= 0 || fsz <= 0)
    {
        return;
    }

    long tail = fsz < 65536 ? fsz : 65536;
    unsigned char* t = read_alloc(f, fsz - tail, (size_t)tail);
    if (!t)
    {
        return;
    }

    for (long i = tail - 14; i >= 0; i--)
    {
        if (!memcmp(t + i, "OggS", 4))
        {
            uint64_t g = le64(t + i + 6);
            if (g != UINT64_MAX && g > preskip)
            {
                emit_duration(c, (double)(g - preskip) / rate);
            }

            break;
        }
    }
    
    free(t);
}

static void read_wav(Ctx* c, FILE* f, long fsz)
{
    long off = 12;
    uint32_t byterate = 0;
    unsigned char h[8];
    while (off + 8 <= fsz && read_at(f, off, h, 8))
    {
        uint32_t len = le32(h + 4);
        if (!memcmp(h, "fmt ", 4) && len >= 16)
        {
            unsigned char fmt[16];
            if (read_at(f, off + 8, fmt, 16))
            {
                byterate = le32(fmt + 8);
            }
        }
        else if (!memcmp(h, "data", 4))
        {
            if (byterate)
            {
                emit_duration(c, (double)len / byterate);
            }
        }
        else if (!memcmp(h, "LIST", 4) && len > 4 && len < MAX_TAG_BYTES)
        {
            unsigned char* b = read_alloc(f, off + 8, len);
            if (b && !memcmp(b, "INFO", 4))
            {
                static const char* map[][2] = {
                    { "INAM", "title" },
                    { "IART", "artist" },
                    { "IPRD", "album" },
                    { "ICRD", "date" },
                    { "IGNR", "genre" },
                    { "ITRK", "tracknumber" },
                    { "ICMT", "comment" },
                };

                for (size_t p = 4; p + 8 <= len;)
                {
                    uint32_t sl = le32(b + p + 4);
                    if (sl > len - p - 8)
                    {
                        break;
                    }

                    for (size_t k = 0; k < sizeof map / sizeof map[0]; k++)
                    {
                        if (!memcmp(b + p, map[k][0], 4))
                        {
                            char* v = 
                                to_utf8(b + p + 8, strnlen((char*)b + p + 8, sl), 3);

                            if (v)
                            {
                                emit(c, map[k][1], v);
                            }

                            g_free(v);
                        }
                    }

                    p += 8 + sl + (sl & 1);
                }
            }

            free(b);
        }
        else if ((!memcmp(h, "id3 ", 4) || !memcmp(h, "ID3 ", 4)) && len < MAX_TAG_BYTES)
        {
            unsigned char* b = read_alloc(f, off + 8, len);
            if (b)
            {
                parse_id3v2(c, b, len);
            }

            free(b);
        }

        off += 8 + (long)len + (len & 1);
    }
}

static bool read_any(const char* path, Ctx* c)
{
    FILE* f = g_fopen(path, "rb");
    if (!f)
    {
        return false;
    }

    long fsz = file_size(f);
    unsigned char magic[12] = { 0 };
    long off = 0;
    size_t id3 = id3v2_size_at(f, 0);
    if (id3)
    {
        unsigned char* b = read_alloc(f, 0, id3);
        if (b)
        {
            parse_id3v2(c, b, id3);
        }

        free(b);
        off = (long)id3;
    }

    if (!read_at(f, off, magic, sizeof magic))
    {
        fclose(f);
        return c->got_text;
    }

    if (!memcmp(magic, "fLaC", 4))
    {
        read_flac(c, f, off);
    }
    else if (!memcmp(magic, "OggS", 4))
    {
        read_ogg(c, f);
    }
    else if (!memcmp(magic, "RIFF", 4) && !memcmp(magic + 8, "WAVE", 4))
    {
        read_wav(c, f, fsz);
    }
    else
    {
        if (!c->got_text)
        {
            read_id3v1(c, f);
        }

        if (!c->want_picture)
        {
            emit_duration(c, mp3_duration(f, off, fsz));
        }
    }

    fclose(f);
    return true;
}

bool tags_read(const char* path, TagFn fn, void* ud)
{
    Ctx c = {
        .fn = fn,
        .ud = ud,
    };

    return read_any(path, &c);
}

bool tags_read_picture(const char* path, unsigned char** data, size_t* len)
{
    Ctx c = {
        .want_picture = true,
        .pic_type = -1,
    };

    read_any(path, &c);
    *data = c.pic;
    *len = c.pic_len;
    return c.pic != NULL;
}
