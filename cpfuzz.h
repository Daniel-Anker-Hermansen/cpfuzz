#include <inttypes.h>
#include <stddef.h>

typedef int64_t i64;

typedef struct context_t context_t;

extern "C" void gen_newline(context_t *context);

extern "C" i64 gen_i64(context_t *context, i64 lower, i64 higher);

extern "C" i64 *gen_i64_array(context_t *context, size_t length, i64 lower, i64 higher);

extern "C" void gen_ascii(context_t *context, char *ascii);
