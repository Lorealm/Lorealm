const jt = "5";
var yt;
typeof window < "u" && ((yt = window.__svelte ?? (window.__svelte = {})).v ?? (yt.v = /* @__PURE__ */ new Set())).add(jt);
let ot = !1, Bt = !1;
function Ut() {
  ot = !0;
}
Ut();
const Vt = 2, g = Symbol(), ht = !1, k = 2, Yt = 4, Et = 8, vt = 16, Y = 32, H = 64, G = 128, x = 256, Z = 512, b = 1024, C = 2048, U = 4096, mt = 8192, tt = 16384, Ht = 32768, Kt = 1 << 19, xt = 1 << 20, at = 1 << 21, ft = Symbol("$state");
var bt = Array.isArray, Wt = Array.prototype.indexOf, zt = Object.defineProperty, it = Object.getOwnPropertyDescriptor, Gt = Object.prototype, Zt = Array.prototype, $t = Object.getPrototypeOf;
function Jt(t) {
  for (var n = 0; n < t.length; n++)
    t[n]();
}
let $ = [];
function Qt() {
  var t = $;
  $ = [], Jt(t);
}
function Xt(t) {
  $.length === 0 && queueMicrotask(Qt), $.push(t);
}
function Tt(t) {
  return t === this.v;
}
function tn(t, n) {
  return t != t ? n == n : t !== n || t !== null && typeof t == "object" || typeof t == "function";
}
function nn(t) {
  return !tn(t, this.v);
}
function rn() {
  throw new Error("https://svelte.dev/e/effect_update_depth_exceeded");
}
function en() {
  throw new Error("https://svelte.dev/e/state_descriptors_fixed");
}
function ln() {
  throw new Error("https://svelte.dev/e/state_prototype_fixed");
}
function un() {
  throw new Error("https://svelte.dev/e/state_unsafe_mutation");
}
function L(t, n) {
  if (typeof t != "object" || t === null || ft in t)
    return t;
  const r = $t(t);
  if (r !== Gt && r !== Zt)
    return t;
  var e = /* @__PURE__ */ new Map(), u = bt(t), l = F(0), _ = c, v = (i) => {
    var f = c;
    A(_);
    var a;
    return a = i(), A(f), a;
  };
  return u && e.set("length", F(
    /** @type {any[]} */
    t.length
  )), new Proxy(
    /** @type {any} */
    t,
    {
      defineProperty(i, f, a) {
        (!("value" in a) || a.configurable === !1 || a.enumerable === !1 || a.writable === !1) && en();
        var o = e.get(f);
        return o === void 0 ? (o = v(() => F(a.value)), e.set(f, o)) : D(
          o,
          v(() => L(a.value))
        ), !0;
      },
      deleteProperty(i, f) {
        var a = e.get(f);
        if (a === void 0)
          f in i && e.set(
            f,
            v(() => F(g))
          );
        else {
          if (u && typeof f == "string") {
            var o = (
              /** @type {Source<number>} */
              e.get("length")
            ), s = Number(f);
            Number.isInteger(s) && s < o.v && D(o, s);
          }
          D(a, g), pt(l);
        }
        return !0;
      },
      get(i, f, a) {
        var E;
        if (f === ft)
          return t;
        var o = e.get(f), s = f in i;
        if (o === void 0 && (!s || (E = it(i, f)) != null && E.writable) && (o = v(() => F(L(s ? i[f] : g))), e.set(f, o)), o !== void 0) {
          var h = j(o);
          return h === g ? void 0 : h;
        }
        return Reflect.get(i, f, a);
      },
      getOwnPropertyDescriptor(i, f) {
        var a = Reflect.getOwnPropertyDescriptor(i, f);
        if (a && "value" in a) {
          var o = e.get(f);
          o && (a.value = j(o));
        } else if (a === void 0) {
          var s = e.get(f), h = s == null ? void 0 : s.v;
          if (s !== void 0 && h !== g)
            return {
              enumerable: !0,
              configurable: !0,
              value: h,
              writable: !0
            };
        }
        return a;
      },
      has(i, f) {
        var h;
        if (f === ft)
          return !0;
        var a = e.get(f), o = a !== void 0 && a.v !== g || Reflect.has(i, f);
        if (a !== void 0 || p !== null && (!o || (h = it(i, f)) != null && h.writable)) {
          a === void 0 && (a = v(() => F(o ? L(i[f]) : g)), e.set(f, a));
          var s = j(a);
          if (s === g)
            return !1;
        }
        return o;
      },
      set(i, f, a, o) {
        var I;
        var s = e.get(f), h = f in i;
        if (u && f === "length")
          for (var E = a; E < /** @type {Source<number>} */
          s.v; E += 1) {
            var O = e.get(E + "");
            O !== void 0 ? D(O, g) : E in i && (O = v(() => F(g)), e.set(E + "", O));
          }
        s === void 0 ? (!h || (I = it(i, f)) != null && I.writable) && (s = v(() => F(void 0)), D(
          s,
          v(() => L(a))
        ), e.set(f, s)) : (h = s.v !== g, D(
          s,
          v(() => L(a))
        ));
        var S = Reflect.getOwnPropertyDescriptor(i, f);
        if (S != null && S.set && S.set.call(o, a), !h) {
          if (u && typeof f == "string") {
            var K = (
              /** @type {Source<number>} */
              e.get("length")
            ), M = Number(f);
            Number.isInteger(M) && M >= K.v && D(K, M + 1);
          }
          pt(l);
        }
        return !0;
      },
      ownKeys(i) {
        j(l);
        var f = Reflect.ownKeys(i).filter((s) => {
          var h = e.get(s);
          return h === void 0 || h.v !== g;
        });
        for (var [a, o] of e)
          o.v !== g && !(a in i) && f.push(a);
        return f;
      },
      setPrototypeOf() {
        ln();
      }
    }
  );
}
function pt(t, n = 1) {
  D(t, t.v + n);
}
const V = /* @__PURE__ */ new Map();
function kt(t, n) {
  var r = {
    f: 0,
    // TODO ideally we could skip this altogether, but it causes type errors
    v: t,
    reactions: null,
    equals: Tt,
    rv: 0,
    wv: 0
  };
  return r;
}
function F(t, n) {
  const r = kt(t);
  return dn(r), r;
}
// @__NO_SIDE_EFFECTS__
function fn(t, n = !1) {
  var e;
  const r = kt(t);
  return n || (r.equals = nn), ot && T !== null && T.l !== null && ((e = T.l).s ?? (e.s = [])).push(r), r;
}
function D(t, n, r = !1) {
  c !== null && !q && ut() && (c.f & (k | vt)) !== 0 && !(d != null && d.includes(t)) && un();
  let e = r ? L(n) : n;
  return an(t, e);
}
function an(t, n) {
  if (!t.equals(n)) {
    var r = t.v;
    nt ? V.set(t, n) : V.set(t, r), t.v = n, t.wv = Ct(), Ot(t, C), ut() && p !== null && (p.f & b) !== 0 && (p.f & (Y | H)) === 0 && (m === null ? gn([t]) : m.push(t));
  }
  return n;
}
function Ot(t, n) {
  var r = t.reactions;
  if (r !== null)
    for (var e = ut(), u = r.length, l = 0; l < u; l++) {
      var _ = r[l], v = _.f;
      (v & C) === 0 && (!e && _ === p || (N(_, n), (v & (b | x)) !== 0 && ((v & k) !== 0 ? Ot(
        /** @type {Derived} */
        _,
        U
      ) : ct(
        /** @type {Effect} */
        _
      ))));
    }
}
// @__NO_SIDE_EFFECTS__
function sn(t) {
  var n = k | C, r = c !== null && (c.f & k) !== 0 ? (
    /** @type {Derived} */
    c
  ) : null;
  return p === null || r !== null && (r.f & x) !== 0 ? n |= x : p.f |= xt, {
    ctx: T,
    deps: null,
    effects: null,
    equals: Tt,
    f: n,
    fn: t,
    reactions: null,
    rv: 0,
    v: (
      /** @type {V} */
      null
    ),
    wv: 0,
    parent: r ?? p
  };
}
function Dt(t) {
  var n = t.effects;
  if (n !== null) {
    t.effects = null;
    for (var r = 0; r < n.length; r += 1)
      lt(
        /** @type {Effect} */
        n[r]
      );
  }
}
function on(t) {
  for (var n = t.parent; n !== null; ) {
    if ((n.f & k) === 0)
      return (
        /** @type {Effect} */
        n
      );
    n = n.parent;
  }
  return null;
}
function vn(t) {
  var n, r = p;
  B(on(t));
  try {
    Dt(t), n = qt(t);
  } finally {
    B(r);
  }
  return n;
}
function Nt(t) {
  var n = vn(t), r = (R || (t.f & x) !== 0) && t.deps !== null ? U : b;
  N(t, r), t.equals(n) || (t.v = n, t.wv = Ct());
}
var _n, cn, hn;
// @__NO_SIDE_EFFECTS__
function Ft(t) {
  return cn.call(t);
}
// @__NO_SIDE_EFFECTS__
function Rt(t) {
  return hn.call(t);
}
function wt(t, n) {
  return /* @__PURE__ */ Ft(t);
}
function pn(t, n = 1, r = !1) {
  let e = t;
  for (; n--; )
    e = /** @type {TemplateNode} */
    /* @__PURE__ */ Rt(e);
  return e;
}
let W = !1, st = !1, J = null, P = !1, nt = !1;
function dt(t) {
  nt = t;
}
let z = [];
let c = null, q = !1;
function A(t) {
  c = t;
}
let p = null;
function B(t) {
  p = t;
}
let d = null;
function wn(t) {
  d = t;
}
function dn(t) {
  c !== null && c.f & at && (d === null ? wn([t]) : d.push(t));
}
let w = null, y = 0, m = null;
function gn(t) {
  m = t;
}
let At = 1, Q = 0, R = !1;
function Ct() {
  return ++At;
}
function rt(t) {
  var o;
  var n = t.f;
  if ((n & C) !== 0)
    return !0;
  if ((n & U) !== 0) {
    var r = t.deps, e = (n & x) !== 0;
    if (r !== null) {
      var u, l, _ = (n & Z) !== 0, v = e && p !== null && !R, i = r.length;
      if (_ || v) {
        var f = (
          /** @type {Derived} */
          t
        ), a = f.parent;
        for (u = 0; u < i; u++)
          l = r[u], (_ || !((o = l == null ? void 0 : l.reactions) != null && o.includes(f))) && (l.reactions ?? (l.reactions = [])).push(f);
        _ && (f.f ^= Z), v && a !== null && (a.f & x) === 0 && (f.f ^= x);
      }
      for (u = 0; u < i; u++)
        if (l = r[u], rt(
          /** @type {Derived} */
          l
        ) && Nt(
          /** @type {Derived} */
          l
        ), l.wv > t.wv)
          return !0;
    }
    (!e || p !== null && !R) && N(t, b);
  }
  return !1;
}
function yn(t, n) {
  for (var r = n; r !== null; ) {
    if ((r.f & G) !== 0)
      try {
        r.fn(t);
        return;
      } catch {
        r.f ^= G;
      }
    r = r.parent;
  }
  throw W = !1, t;
}
function En(t) {
  return (t.f & tt) === 0 && (t.parent === null || (t.parent.f & G) === 0);
}
function et(t, n, r, e) {
  if (W) {
    if (r === null && (W = !1), En(n))
      throw t;
    return;
  }
  r !== null && (W = !0);
  {
    yn(t, n);
    return;
  }
}
function It(t, n, r = !0) {
  var e = t.reactions;
  if (e !== null)
    for (var u = 0; u < e.length; u++) {
      var l = e[u];
      d != null && d.includes(t) || ((l.f & k) !== 0 ? It(
        /** @type {Derived} */
        l,
        n,
        !1
      ) : n === l && (r ? N(l, C) : (l.f & b) !== 0 && N(l, U), ct(
        /** @type {Effect} */
        l
      )));
    }
}
function qt(t) {
  var h;
  var n = w, r = y, e = m, u = c, l = R, _ = d, v = T, i = q, f = t.f;
  w = /** @type {null | Value[]} */
  null, y = 0, m = null, R = (f & x) !== 0 && (q || !P || c === null), c = (f & (Y | H)) === 0 ? t : null, d = null, gt(t.ctx), q = !1, Q++, t.f |= at;
  try {
    var a = (
      /** @type {Function} */
      (0, t.fn)()
    ), o = t.deps;
    if (w !== null) {
      var s;
      if (X(t, y), o !== null && y > 0)
        for (o.length = y + w.length, s = 0; s < w.length; s++)
          o[y + s] = w[s];
      else
        t.deps = o = w;
      if (!R)
        for (s = y; s < o.length; s++)
          ((h = o[s]).reactions ?? (h.reactions = [])).push(t);
    } else o !== null && y < o.length && (X(t, y), o.length = y);
    if (ut() && m !== null && !q && o !== null && (t.f & (k | U | C)) === 0)
      for (s = 0; s < /** @type {Source[]} */
      m.length; s++)
        It(
          m[s],
          /** @type {Effect} */
          t
        );
    return u !== null && (Q++, m !== null && (e === null ? e = m : e.push(.../** @type {Source[]} */
    m))), a;
  } finally {
    w = n, y = r, m = e, c = u, R = l, d = _, gt(v), q = i, t.f ^= at;
  }
}
function mn(t, n) {
  let r = n.reactions;
  if (r !== null) {
    var e = Wt.call(r, t);
    if (e !== -1) {
      var u = r.length - 1;
      u === 0 ? r = n.reactions = null : (r[e] = r[u], r.pop());
    }
  }
  r === null && (n.f & k) !== 0 && // Destroying a child effect while updating a parent effect can cause a dependency to appear
  // to be unused, when in fact it is used by the currently-updating parent. Checking `new_deps`
  // allows us to skip the expensive work of disconnecting and immediately reconnecting it
  (w === null || !w.includes(n)) && (N(n, U), (n.f & (x | Z)) === 0 && (n.f ^= Z), Dt(
    /** @type {Derived} **/
    n
  ), X(
    /** @type {Derived} **/
    n,
    0
  ));
}
function X(t, n) {
  var r = t.deps;
  if (r !== null)
    for (var e = n; e < r.length; e++)
      mn(t, r[e]);
}
function _t(t) {
  var n = t.f;
  if ((n & tt) === 0) {
    N(t, b);
    var r = p, e = T, u = P;
    p = t, P = !0;
    try {
      (n & vt) !== 0 ? An(t) : Mt(t), St(t);
      var l = qt(t);
      t.teardown = typeof l == "function" ? l : null, t.wv = At;
      var _ = t.deps, v;
      ht && Bt && t.f & C;
    } catch (i) {
      et(i, t, r, e || t.ctx);
    } finally {
      P = u, p = r;
    }
  }
}
function xn() {
  try {
    rn();
  } catch (t) {
    if (J !== null)
      et(t, J, null);
    else
      throw t;
  }
}
function bn() {
  var t = P;
  try {
    var n = 0;
    for (P = !0; z.length > 0; ) {
      n++ > 1e3 && xn();
      var r = z, e = r.length;
      z = [];
      for (var u = 0; u < e; u++) {
        var l = kn(r[u]);
        Tn(l);
      }
      V.clear();
    }
  } finally {
    st = !1, P = t, J = null;
  }
}
function Tn(t) {
  var n = t.length;
  if (n !== 0)
    for (var r = 0; r < n; r++) {
      var e = t[r];
      if ((e.f & (tt | mt)) === 0)
        try {
          rt(e) && (_t(e), e.deps === null && e.first === null && e.nodes_start === null && (e.teardown === null ? Lt(e) : e.fn = null));
        } catch (u) {
          et(u, e, null, e.ctx);
        }
    }
}
function ct(t) {
  st || (st = !0, queueMicrotask(bn));
  for (var n = J = t; n.parent !== null; ) {
    n = n.parent;
    var r = n.f;
    if ((r & (H | Y)) !== 0) {
      if ((r & b) === 0) return;
      n.f ^= b;
    }
  }
  z.push(n);
}
function kn(t) {
  for (var n = [], r = t; r !== null; ) {
    var e = r.f, u = (e & (Y | H)) !== 0, l = u && (e & b) !== 0;
    if (!l && (e & mt) === 0) {
      if ((e & Yt) !== 0)
        n.push(r);
      else if (u)
        r.f ^= b;
      else {
        var _ = c;
        try {
          c = r, rt(r) && _t(r);
        } catch (f) {
          et(f, r, null, r.ctx);
        } finally {
          c = _;
        }
      }
      var v = r.first;
      if (v !== null) {
        r = v;
        continue;
      }
    }
    var i = r.parent;
    for (r = r.next; r === null && i !== null; )
      r = i.next, i = i.parent;
  }
  return n;
}
function j(t) {
  var n = t.f, r = (n & k) !== 0;
  if (c !== null && !q) {
    if (!(d != null && d.includes(t))) {
      var e = c.deps;
      t.rv < Q && (t.rv = Q, w === null && e !== null && e[y] === t ? y++ : w === null ? w = [t] : (!R || !w.includes(t)) && w.push(t));
    }
  } else if (r && /** @type {Derived} */
  t.deps === null && /** @type {Derived} */
  t.effects === null) {
    var u = (
      /** @type {Derived} */
      t
    ), l = u.parent;
    l !== null && (l.f & x) === 0 && (u.f ^= x);
  }
  return r && (u = /** @type {Derived} */
  t, rt(u) && Nt(u)), nt && V.has(t) ? V.get(t) : t.v;
}
const On = -7169;
function N(t, n) {
  t.f = t.f & On | n;
}
function Dn(t, n) {
  var r = n.last;
  r === null ? n.last = n.first = t : (r.next = t, t.prev = r, n.last = t);
}
function Pt(t, n, r, e = !0) {
  var u = p, l = {
    ctx: T,
    deps: null,
    nodes_start: null,
    nodes_end: null,
    f: t | C,
    first: null,
    fn: n,
    last: null,
    next: null,
    parent: u,
    prev: null,
    teardown: null,
    transitions: null,
    wv: 0
  };
  if (r)
    try {
      _t(l), l.f |= Ht;
    } catch (i) {
      throw lt(l), i;
    }
  else n !== null && ct(l);
  var _ = r && l.deps === null && l.first === null && l.nodes_start === null && l.teardown === null && (l.f & (xt | G)) === 0;
  if (!_ && e && (u !== null && Dn(l, u), c !== null && (c.f & k) !== 0)) {
    var v = (
      /** @type {Derived} */
      c
    );
    (v.effects ?? (v.effects = [])).push(l);
  }
  return l;
}
function Nn(t) {
  const n = Pt(Et, null, !1);
  return N(n, b), n.teardown = t, n;
}
function Fn(t, n = [], r = sn) {
  const e = n.map(r);
  return Rn(() => t(...e.map(j)));
}
function Rn(t, n = 0) {
  return Pt(Et | vt | n, t, !0);
}
function St(t) {
  var n = t.teardown;
  if (n !== null) {
    const r = nt, e = c;
    dt(!0), A(null);
    try {
      n.call(null);
    } finally {
      dt(r), A(e);
    }
  }
}
function Mt(t, n = !1) {
  var r = t.first;
  for (t.first = t.last = null; r !== null; ) {
    var e = r.next;
    (r.f & H) !== 0 ? r.parent = null : lt(r, n), r = e;
  }
}
function An(t) {
  for (var n = t.first; n !== null; ) {
    var r = n.next;
    (n.f & Y) === 0 && lt(n), n = r;
  }
}
function lt(t, n = !0) {
  var r = !1;
  if ((n || (t.f & Kt) !== 0) && t.nodes_start !== null) {
    for (var e = t.nodes_start, u = t.nodes_end; e !== null; ) {
      var l = e === u ? null : (
        /** @type {TemplateNode} */
        /* @__PURE__ */ Rt(e)
      );
      e.remove(), e = l;
    }
    r = !0;
  }
  Mt(t, n && !r), X(t, 0), N(t, tt);
  var _ = t.transitions;
  if (_ !== null)
    for (const i of _)
      i.stop();
  St(t);
  var v = t.parent;
  v !== null && v.first !== null && Lt(t), t.next = t.prev = t.teardown = t.ctx = t.deps = t.fn = t.nodes_start = t.nodes_end = null;
}
function Lt(t) {
  var n = t.parent, r = t.prev, e = t.next;
  r !== null && (r.next = e), e !== null && (e.prev = r), n !== null && (n.first === t && (n.first = e), n.last === t && (n.last = r));
}
let T = null;
function gt(t) {
  T = t;
}
function ut() {
  return !ot || T !== null && T.l === null;
}
function Cn(t) {
  var n = c, r = p;
  A(null), B(null);
  try {
    return t();
  } finally {
    A(n), B(r);
  }
}
function In(t, n, r, e = {}) {
  function u(l) {
    if (e.capture || Pn.call(n, l), !l.cancelBubble)
      return Cn(() => r == null ? void 0 : r.call(this, l));
  }
  return t.startsWith("pointer") || t.startsWith("touch") || t === "wheel" ? Xt(() => {
    n.addEventListener(t, u, e);
  }) : n.addEventListener(t, u, e), u;
}
function qn(t, n, r, e, u) {
  var l = { capture: e, passive: u }, _ = In(t, n, r, l);
  (n === document.body || n === window || n === document) && Nn(() => {
    n.removeEventListener(t, _, l);
  });
}
function Pn(t) {
  var M;
  var n = this, r = (
    /** @type {Node} */
    n.ownerDocument
  ), e = t.type, u = ((M = t.composedPath) == null ? void 0 : M.call(t)) || [], l = (
    /** @type {null | Element} */
    u[0] || t.target
  ), _ = 0, v = t.__root;
  if (v) {
    var i = u.indexOf(v);
    if (i !== -1 && (n === document || n === /** @type {any} */
    window)) {
      t.__root = n;
      return;
    }
    var f = u.indexOf(n);
    if (f === -1)
      return;
    i <= f && (_ = i);
  }
  if (l = /** @type {Element} */
  u[_] || t.target, l !== n) {
    zt(t, "currentTarget", {
      configurable: !0,
      get() {
        return l || r;
      }
    });
    var a = c, o = p;
    A(null), B(null);
    try {
      for (var s, h = []; l !== null; ) {
        var E = l.assignedSlot || l.parentNode || /** @type {any} */
        l.host || null;
        try {
          var O = l["__" + e];
          if (O != null && (!/** @type {any} */
          l.disabled || // DOM could've been updated already by the time this is reached, so we check this as well
          // -> the target could not have been disabled because it emits the event in the first place
          t.target === l))
            if (bt(O)) {
              var [S, ...K] = O;
              S.apply(l, [t, ...K]);
            } else
              O.call(l, t);
        } catch (I) {
          s ? h.push(I) : s = I;
        }
        if (t.cancelBubble || E === n || E === null)
          break;
        l = E;
      }
      if (s) {
        for (let I of h)
          queueMicrotask(() => {
            throw I;
          });
        throw s;
      }
    } finally {
      t.__root = n, delete t.currentTarget, A(a), B(o);
    }
  }
}
function Sn(t) {
  var n = document.createElement("template");
  return n.innerHTML = t, n.content;
}
function Mn(t, n) {
  var r = (
    /** @type {Effect} */
    p
  );
  r.nodes_start === null && (r.nodes_start = t, r.nodes_end = n);
}
// @__NO_SIDE_EFFECTS__
function Ln(t, n) {
  var r = (n & Vt) !== 0, e, u = !t.startsWith("<!>");
  return () => {
    e === void 0 && (e = Sn(u ? t : "<!>" + t), e = /** @type {Node} */
    /* @__PURE__ */ Ft(e));
    var l = (
      /** @type {TemplateNode} */
      r || _n ? document.importNode(e, !0) : e.cloneNode(!0)
    );
    return Mn(l, l), l;
  };
}
function jn(t, n) {
  t !== null && t.before(
    /** @type {Node} */
    n
  );
}
function Bn(t, n) {
  var r = n == null ? "" : typeof n == "object" ? n + "" : n;
  r !== (t.__t ?? (t.__t = t.nodeValue)) && (t.__t = r, t.nodeValue = r + "");
}
var Un = /* @__PURE__ */ Ln('<div class="component svelte-17qmz8o"><input type="text"> <h1 class="svelte-17qmz8o"> </h1></div>');
function Vn(t) {
  let n = /* @__PURE__ */ fn("");
  function r(v) {
    D(n, v.target.value);
  }
  var e = Un(), u = wt(e), l = pn(u, 2), _ = wt(l);
  Fn(() => Bn(_, j(n))), qn("input", u, r), jn(t, e);
}
export {
  Vn as default
};
