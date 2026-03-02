const { RawServer } = require('..')

const url = process.argv[2] || 'tcp://127.0.0.1:24002'
const method = process.argv[3] || 'Raw/Echo'

RawServer.listen(url, method, (payload) => Buffer.from(payload))

setInterval(() => {}, 1 << 30)
