import test from 'ava'

import { createForySerializer, plus100, Type } from '../index'

test('sync function from native code', (t) => {
  const fixture = 42
  t.is(plus100(fixture), fixture + 100)
})

test('fory serializer roundtrip', (t) => {
  const serializer = createForySerializer(
    Type.object('test-message', {
      id: Type.string(),
      count: Type.uint16(),
    }),
  )
  const payload = { id: 'ping', count: 12 }
  const encoded = serializer.serialize(payload)
  t.deepEqual(serializer.deserialize(encoded), payload)
})
