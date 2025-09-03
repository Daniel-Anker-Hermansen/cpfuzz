#include <inttypes.h>
#include <stddef.h>

typedef uint8_t u8;
typedef int64_t i64;

typedef struct context_state_t context_state_t;

typedef struct context_t {
	void (*gen_new_line)(context_state_t*);
	i64 (*gen_i64)(context_state_t*, i64, i64);
	i64* (*gen_i64_array)(context_state_t*, size_t, i64, i64);
	void (*gen_ascii)(context_state_t*, char*);
	context_state_t * context_state;
} context_t;


void gen_newline(context_t *context) {
	context->gen_new_line(context->context_state);
}

i64 gen_i64(context_t *context, i64 lower, i64 higher) {
	return context->gen_i64(context->context_state, lower, higher);
}

i64 *gen_i64_array(context_t *context, size_t length, i64 lower, i64 higher) {
	return context->gen_i64_array(context->context_state, length, lower, higher);
}

void gen_ascii(context_t *context, char *ascii) {
	context->gen_ascii(context->context_state, ascii);
}
