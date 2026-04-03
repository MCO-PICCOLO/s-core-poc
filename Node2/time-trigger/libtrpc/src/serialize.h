#ifndef _SERIALIZE_H_
#define _SERIALIZE_H_

#ifdef __cplusplus
extern "C" {
#endif

#define INITIAL_SERIAL_BUF_SIZE		32

#define DATATYPE(_size)							\
		typeof( __builtin_choose_expr(_size == 1, (uint8_t)1,	\
			__builtin_choose_expr(_size == 2, (uint16_t)2,	\
			__builtin_choose_expr(_size == 4, (uint32_t)4,	\
			__builtin_choose_expr(_size == 8, (uint64_t)8,	\
							(void)-1)))))

#define DEFINE_SERIALIZE_T(_type)					\
	int serialize_##_type(serial_buf_t *b, _type t)			\
	{								\
		DATATYPE(sizeof(_type)) x;				\
		if (__builtin_types_compatible_p(_type, float))		\
			x = htonf(t);					\
		else if (__builtin_types_compatible_p(_type, double))	\
			x = htond(t);					\
		else {							\
			switch (sizeof(x)) {				\
			case 1:						\
				x = t;					\
				break;					\
			case 2:						\
				x = htons(t);				\
				break;					\
			case 4:						\
				x = htonl(t);				\
				break;					\
			case 8:						\
				x = htonll(t);				\
				break;					\
			}						\
		}							\
		if (reserve_space(b, sizeof(x))) return -1;		\
		memcpy((char *)b->data + b->pos, &x, sizeof(x));	\
		b->pos += sizeof(x);					\
		return 0;						\
	}

#define DEFINE_DESERIALIZE_T(_type)					\
	int deserialize_##_type(serial_buf_t *b, _type *t)		\
	{								\
		DATATYPE(sizeof(_type)) x;				\
		if (b->pos < sizeof(x)) return -1;			\
		b->pos -= sizeof(x);					\
		memcpy(&x, (char *)b->data + b->pos, sizeof(x));	\
		if (__builtin_types_compatible_p(_type, float))		\
			*t = ntohf(x);					\
		else if (__builtin_types_compatible_p(_type, double))	\
			*t = ntohd(x);					\
		else {							\
			switch (sizeof(x)) {				\
			case 1:						\
				*t = x;					\
				break;					\
			case 2:						\
				*t = ntohs(x);				\
				break;					\
			case 4:						\
				*t = ntohl(x);				\
				break;					\
			case 8:						\
				*t = ntohll(x);				\
				break;					\
			}						\
		}							\
		return 0;						\
	}

#define DEFINE_SERIALIZE(_type)						\
	DEFINE_SERIALIZE_T(_type)					\
	DEFINE_DESERIALIZE_T(_type)					\

#ifdef __cplusplus
}
#endif

#endif	/* _SERIALIZE_H_ */
