for one in (1, 'a', [1], (1,), {1: 2}):
  try:
    vars(one)
  except TypeError as exc:
    print(repr(one), '->', exc)
import sys
class Foo:
  x = 1
f = Foo()
f.y = 2
print(sorted(vars(Foo)) == sorted(vars(Foo)), vars(f))
d = vars(f)
d['z'] = 3
print(f.z if hasattr(f, 'z') else 'copy, not live')
