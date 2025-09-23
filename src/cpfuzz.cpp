#include <inttypes.h>
#include <stddef.h>
#include <vector>

typedef int64_t i64;

typedef struct context_state_t context_state_t;

typedef struct context_t {
	void (*write_nl)(context_state_t*);
	void (*write_i64)(context_state_t*, i64);
	void (*write_ascii)(context_state_t*, char*);
	i64 (*rand_i64)(i64, i64);
	context_state_t * context_state;
} context_t;

void generate(context_t *context);

extern "C" void __generate(context_t *context) {
	generate(context);
}

void write_nl(context_t *context) {
	context->write_nl(context->context_state);
}

void write_i64(context_t *context, i64 val) {
	context->write_i64(context->context_state, val);
}

void write_ascii(context_t *context, char *val) {
	context->write_ascii(context->context_state, val);
}

i64 rand_i64(context_t *context, i64 lower, i64 higher) {
	return context->rand_i64(lower, higher);
}

std::vector<i64> rand_i64_array(context_t *context, i64 length, i64 lower, i64 higher) {
	std::vector<i64> res(length);
	for (i64 i = 0; i < length; i++) res[i] = rand_i64(context, lower, higher);
	return res;
}
