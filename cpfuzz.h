#include <inttypes.h>
#include <stddef.h>
#include <vector>

typedef int64_t i64;

typedef struct context_t context_t;

void write_nl(context_t *context);

void write_i64(context_t *context, i64);

void write_ascii(context_t *context, char*);

i64 rand_i64(context_t *context, i64 lower, i64 higher);

template<class It>
void write_i64_seq(context_t *context, It first, It end) {
	while (first != end) write_i64(context, *first), first++;
}


std::vector<i64> rand_i64_array(context_t *context, i64 length, i64 lower, i64 higher);
