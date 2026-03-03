import test from 'ava'

import { plus100, AsyncDealer, AsyncRouter } from '../index'

test('sync function from native code', (t) => {
  const fixture = 42
  t.is(plus100(fixture), fixture + 100)
})

test('AsyncDealer and AsyncRouter basic communication', async (t) => {
  const url = `inproc://test_${process.pid}_${Date.now()}`

  // Start the router (server-side)
  const router = await AsyncRouter.listen(url)

  // Start the dealer (client-side)
  const dealer = await AsyncDealer.dial(url)

  t.teardown(() => {
    dealer.close()
    router.close()
  })

  // Client sends a message
  const requestData = Buffer.from('Hello from client')
  const requestId = await dealer.send(requestData)
  t.truthy(requestId, 'Request ID should be returned')

  // Server receives the message
  const receivedMsg = await router.recv()
  t.deepEqual(receivedMsg.body(), requestData, 'Server should receive the correct message')
  t.truthy(receivedMsg.header().length > 0, 'Message should have a header for routing')

  // Server sends a response using the same header for routing
  const responseData = Buffer.from('Hello from server')
  const responseMsg = receivedMsg.createResponse(responseData)
  await router.send(responseMsg)

  // Client receives the response
  const responseReceived = await dealer.recv()
  t.deepEqual(responseReceived.body(), responseData, 'Client should receive the correct response')
})
