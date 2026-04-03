#include <stdlib.h>	// malloc(), free()
#include <string.h>	// memcpy()
#include <arpa/inet.h>	// htonl(), htons()

#include <endian.h>	// __BYTE_ORDER
#include <byteswap.h>	// __bswap_64()

#include "libtrpc.h"
#include "serialize.h"

/*
 * Extra utility functions for byte-order conversion
 */

static inline uint64_t htonll(uint64_t val)
{
	if (__BYTE_ORDER == __LITTLE_ENDIAN)
		return bswap_64(val);
	else
		return val;
}

static inline uint64_t ntohll(uint64_t val)
{
	if (__BYTE_ORDER == __LITTLE_ENDIAN)
		return bswap_64(val);
	else
		return val;
}

static inline uint32_t htonf(float val)
{
	if (__BYTE_ORDER == __LITTLE_ENDIAN)
		return bswap_32(*((uint32_t *)&val));
	else
		return *((uint32_t *)&val);
}

static inline float ntohf(uint32_t val)
{
	float t;
	if (__BYTE_ORDER == __LITTLE_ENDIAN)
		*((uint32_t *)&t) = bswap_32(val);
	else
		*((uint32_t *)&t) = val;
	return t;
}

static inline uint64_t htond(double val)
{
	if (__BYTE_ORDER == __LITTLE_ENDIAN)
		return bswap_64(*((uint64_t *)&val));
	else
		return *((uint64_t *)&val);
}

static inline double ntohd(uint64_t val)
{
	double t;
	if (__BYTE_ORDER == __LITTLE_ENDIAN)
		*((uint64_t *)&t) = bswap_64(val);
	else
		*((uint64_t *)&t) = val;
	return t;
}

/*
 * Data serialization & deserialization
 */

serial_buf_t *new_serial_buf(size_t size)
{
	serial_buf_t *b;

	b = malloc(sizeof(serial_buf_t));
	if (!b) return NULL;

	if (size <= 0)
		size = INITIAL_SERIAL_BUF_SIZE;

	b->data = malloc(size);
	if (!b->data) {
		free(b);
		return NULL;
	}
	b->size = size;
	b->pos = 0;

	return b;
}

serial_buf_t *make_serial_buf(void *data, size_t size)
{
	serial_buf_t *b;

	b = malloc(sizeof(serial_buf_t));
	if (!b) return NULL;

	b->data = data;
	b->size = size;
	b->pos = size;

	return b;
}

void free_serial_buf(serial_buf_t *b)
{
	if (b) {
		free(b->data);
		free(b);
	}
}

void reset_serial_buf(serial_buf_t *b)
{
	if (b) {
		b->pos = 0;
	}
}

static int reserve_space(serial_buf_t *b, size_t bytes)
{
	if (b == NULL) return -1;
	if ((b->pos + bytes) > b->size) {
		void *data;
		size_t new_size;

		new_size = b->size * 2;
		if ((b->pos + bytes) > new_size) new_size += bytes;

		data = realloc(b->data, new_size);
		if (!data) return -1;

		b->data = data;
		b->size = new_size;
	}
	return 0;
}

DEFINE_SERIALIZE(int8_t)
DEFINE_SERIALIZE(int16_t)
DEFINE_SERIALIZE(int32_t)
DEFINE_SERIALIZE(int64_t)
DEFINE_SERIALIZE(float)
DEFINE_SERIALIZE(double)

int serialize_blob(serial_buf_t *b, const char *t, uint32_t bytes)
{
	if (reserve_space(b, bytes + sizeof(bytes))) return -1;

	memcpy((char *)b->data + b->pos, t, bytes);
	b->pos += bytes;

	bytes = htonl(bytes);
	memcpy((char *)b->data + b->pos, &bytes, sizeof(bytes));
	b->pos += sizeof(bytes);

	return 0;
}

int deserialize_blob(serial_buf_t *b, char *t, uint32_t *bytes)
{
	if (b->pos < sizeof(*bytes)) return -1;
	b->pos -= sizeof(*bytes);

	memcpy(bytes, (char *)b->data + b->pos, sizeof(*bytes));
	*bytes = ntohl(*bytes);

	if (b->pos < *bytes) return -1;
	b->pos -= *bytes;
	memcpy(t, (char *)b->data + b->pos, *bytes);

	return 0;
}

int serialize_str(serial_buf_t *b, const char *t)
{
	return serialize_blob(b, t, strlen(t));
}

int deserialize_str(serial_buf_t *b, char *t)
{
	uint32_t len;

	if (deserialize_blob(b, t, &len)) {
		return -1;
	}
	t[len] = '\0';
	return 0;
}
