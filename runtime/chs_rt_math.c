/* Path A M166: IEEE-754 double math floor (math.h). No decimal type.
 * Free names sin/cos/ln/exp/sqrt/pow lower to oo_* here. */
#include "chs_rt.h"
#include <math.h>

double oo_sin(double x) { return sin(x); }
double oo_cos(double x) { return cos(x); }
/* Natural log (ln); domain residual = host math.h (NaN/inf on ≤0). */
double oo_ln(double x) { return log(x); }
double oo_exp(double x) { return exp(x); }
double oo_sqrt(double x) { return sqrt(x); }
double oo_pow(double base, double expn) { return pow(base, expn); }

void oo_print_double(double x) { printf("%g", x); }
