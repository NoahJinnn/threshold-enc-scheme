#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {

char *init(const char *c_init_dkg_json);

char *commit(const char *c_commit_json);

} // extern "C"
