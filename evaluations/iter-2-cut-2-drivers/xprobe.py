"""Tiny X11 client via ctypes (libX11, libXtst): no install needed.

usage (DISPLAY must be set):
  xprobe.py tree                      list mapped top-level windows: id, name, geometry
  xprobe.py type <text>               XTest: type ASCII text (with shift for upper case / symbols)
  xprobe.py key <keysym-name> [mods]  XTest: press+release a key, mods e.g. ctrl,shift
  xprobe.py click <x> <y>             XTest: move pointer and click button 1 (root coordinates)
  xprobe.py close                     send WM_DELETE_WINDOW to every top-level window with a name
  xprobe.py focus                     XSetInputFocus to the first named top-level window
"""
import ctypes, ctypes.util, sys, time

x = ctypes.CDLL("libX11.so.6")
xt = ctypes.CDLL("libXtst.so.6")
x.XOpenDisplay.restype = ctypes.c_void_p
x.XOpenDisplay.argtypes = [ctypes.c_char_p]
x.XDefaultRootWindow.restype = ctypes.c_ulong
x.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
x.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong),
                         ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)), ctypes.POINTER(ctypes.c_uint)]
x.XFetchName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_char_p)]
x.XStringToKeysym.restype = ctypes.c_ulong
x.XStringToKeysym.argtypes = [ctypes.c_char_p]
x.XKeysymToKeycode.restype = ctypes.c_ubyte
x.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
x.XFlush.argtypes = [ctypes.c_void_p]
x.XSync.argtypes = [ctypes.c_void_p, ctypes.c_int]
x.XInternAtom.restype = ctypes.c_ulong
x.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
x.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
xt.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
xt.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
xt.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_ulong]


class XWindowAttributes(ctypes.Structure):
    _fields_ = [("x", ctypes.c_int), ("y", ctypes.c_int), ("width", ctypes.c_int), ("height", ctypes.c_int),
                ("border_width", ctypes.c_int), ("depth", ctypes.c_int), ("visual", ctypes.c_void_p),
                ("root", ctypes.c_ulong), ("class", ctypes.c_int), ("bit_gravity", ctypes.c_int),
                ("win_gravity", ctypes.c_int), ("backing_store", ctypes.c_int), ("backing_planes", ctypes.c_ulong),
                ("backing_pixel", ctypes.c_ulong), ("save_under", ctypes.c_int), ("colormap", ctypes.c_ulong),
                ("map_installed", ctypes.c_int), ("map_state", ctypes.c_int), ("all_event_masks", ctypes.c_long),
                ("your_event_mask", ctypes.c_long), ("do_not_propagate_mask", ctypes.c_long),
                ("override_redirect", ctypes.c_int), ("screen", ctypes.c_void_p)]


x.XGetWindowAttributes.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(XWindowAttributes)]


class XClientMessageEvent(ctypes.Structure):
    _fields_ = [("type", ctypes.c_int), ("serial", ctypes.c_ulong), ("send_event", ctypes.c_int),
                ("display", ctypes.c_void_p), ("window", ctypes.c_ulong), ("message_type", ctypes.c_ulong),
                ("format", ctypes.c_int), ("l", ctypes.c_long * 5)]


class XEvent(ctypes.Union):
    _fields_ = [("xclient", XClientMessageEvent), ("pad", ctypes.c_long * 24)]


x.XSendEvent.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_long, ctypes.POINTER(XEvent)]

d = x.XOpenDisplay(None)
if not d:
    print("cannot open display"); sys.exit(2)
root = x.XDefaultRootWindow(d)


def tops():
    r, p = ctypes.c_ulong(), ctypes.c_ulong()
    ch = ctypes.POINTER(ctypes.c_ulong)()
    n = ctypes.c_uint()
    x.XQueryTree(d, root, ctypes.byref(r), ctypes.byref(p), ctypes.byref(ch), ctypes.byref(n))
    out = []
    for i in range(n.value):
        w = ch[i]
        name = ctypes.c_char_p()
        x.XFetchName(d, w, ctypes.byref(name))
        a = XWindowAttributes()
        x.XGetWindowAttributes(d, w, ctypes.byref(a))
        out.append((w, name.value.decode("utf-8", "replace") if name.value else None, a.x, a.y, a.width, a.height, a.map_state))
    return out


def kc(name):
    return x.XKeysymToKeycode(d, x.XStringToKeysym(name.encode()))


SYM = {" ": "space", "/": "slash", ".": "period", "_": "underscore", "(": "parenleft", ")": "parenright",
       "{": "braceleft", "}": "braceright", ";": "semicolon", ":": "colon", "=": "equal", ",": "comma",
       "-": "minus", '"': "quotedbl", "'": "apostrophe"}
SHIFTED = set('_(){}:"') | set("ABCDEFGHIJKLMNOPQRSTUVWXYZ")


def key(name, mods=()):
    for m in mods:
        xt.XTestFakeKeyEvent(d, kc(m), 1, 0)
    xt.XTestFakeKeyEvent(d, kc(name), 1, 0)
    xt.XTestFakeKeyEvent(d, kc(name), 0, 0)
    for m in reversed(mods):
        xt.XTestFakeKeyEvent(d, kc(m), 0, 0)
    x.XFlush(d)
    time.sleep(0.05)


cmd = sys.argv[1]
if cmd == "tree":
    for t in tops():
        if t[6] == 2:  # IsViewable
            print("window 0x%x name=%r x=%d y=%d w=%d h=%d" % t[:6])
elif cmd == "type":
    for ch in sys.argv[2]:
        name = SYM.get(ch, ch)
        key(name, ["Shift_L"] if ch in SHIFTED else [])
elif cmd == "key":
    mods = [{"ctrl": "Control_L", "shift": "Shift_L", "alt": "Alt_L"}[m] for m in sys.argv[3].split(",")] if len(sys.argv) > 3 else []
    key(sys.argv[2], mods)
elif cmd == "click":
    xt.XTestFakeMotionEvent(d, -1, int(sys.argv[2]), int(sys.argv[3]), 0)
    x.XFlush(d); time.sleep(0.1)
    xt.XTestFakeButtonEvent(d, 1, 1, 0); x.XFlush(d); time.sleep(0.05)
    xt.XTestFakeButtonEvent(d, 1, 0, 0); x.XFlush(d); time.sleep(0.1)
elif cmd == "wheel":  # wheel <x> <y> <n> <up|down>: n clicks of button 4 (up) or 5 (down)
    xt.XTestFakeMotionEvent(d, -1, int(sys.argv[2]), int(sys.argv[3]), 0)
    x.XFlush(d); time.sleep(0.1)
    b = 4 if sys.argv[5] == "up" else 5
    for _ in range(int(sys.argv[4])):
        xt.XTestFakeButtonEvent(d, b, 1, 0); xt.XTestFakeButtonEvent(d, b, 0, 0); x.XFlush(d); time.sleep(0.05)
elif cmd == "focus":
    for t in tops():
        if t[1] and t[6] == 2:
            x.XSetInputFocus(d, t[0], 1, 0); x.XFlush(d); print("focused 0x%x" % t[0]); break
elif cmd == "close":
    wmp = x.XInternAtom(d, b"WM_PROTOCOLS", 0)
    wdw = x.XInternAtom(d, b"WM_DELETE_WINDOW", 0)
    for t in tops():
        if t[1]:
            ev = XEvent()
            ev.xclient.type = 33  # ClientMessage
            ev.xclient.window = t[0]
            ev.xclient.message_type = wmp
            ev.xclient.format = 32
            ev.xclient.l[0] = wdw
            ev.xclient.l[1] = 0
            x.XSendEvent(d, t[0], 0, 0, ctypes.byref(ev))
            print("close sent to 0x%x %r" % (t[0], t[1]))
    x.XFlush(d)
x.XSync(d, 0)
