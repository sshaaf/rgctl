/* CFG feature probes for rgctl expected-facts (C lowering coverage). */

int cfg_short_circuit(int a, int b) {
    if (a && b) {
        return 1;
    }
    return 0;
}

int cfg_do_while(int n) {
    int i = 0;
    int total = 0;
    do {
        total += i;
        i++;
    } while (i < n);
    return total;
}

int cfg_goto_label(int x) {
    if (x < 0) {
        goto done;
    }
    x++;
done:
    return x;
}

int cfg_switch_fallthrough(int x) {
    int r = 0;
    switch (x) {
    case 1:
        r += 1;
        /* fall through */
    case 2:
        r += 2;
        break;
    default:
        r = 0;
        break;
    }
    return r;
}

int cfg_ternary(int x) {
    return x > 0 ? x : -x;
}
