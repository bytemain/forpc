/*eslint-disable block-scoped-var, id-length, no-control-regex, no-magic-numbers, no-prototype-builtins, no-redeclare, no-shadow, no-var, sort-vars*/
"use strict";

var $protobuf = require("protobufjs/minimal");

// Common aliases
var $Reader = $protobuf.Reader, $Writer = $protobuf.Writer, $util = $protobuf.util;

// Exported root namespace
var $root = $protobuf.roots["default"] || ($protobuf.roots["default"] = {});

$root.forpc = (function() {

    /**
     * Namespace forpc.
     * @exports forpc
     * @namespace
     */
    var forpc = {};

    /**
     * StatusCode enum.
     * @name forpc.StatusCode
     * @enum {number}
     * @property {number} OK=0 OK value
     * @property {number} CANCELLED=1 CANCELLED value
     * @property {number} UNKNOWN=2 UNKNOWN value
     * @property {number} INVALID_ARGUMENT=3 INVALID_ARGUMENT value
     * @property {number} DEADLINE_EXCEEDED=4 DEADLINE_EXCEEDED value
     * @property {number} NOT_FOUND=5 NOT_FOUND value
     * @property {number} ALREADY_EXISTS=6 ALREADY_EXISTS value
     * @property {number} PERMISSION_DENIED=7 PERMISSION_DENIED value
     * @property {number} RESOURCE_EXHAUSTED=8 RESOURCE_EXHAUSTED value
     * @property {number} FAILED_PRECONDITION=9 FAILED_PRECONDITION value
     * @property {number} ABORTED=10 ABORTED value
     * @property {number} OUT_OF_RANGE=11 OUT_OF_RANGE value
     * @property {number} UNIMPLEMENTED=12 UNIMPLEMENTED value
     * @property {number} INTERNAL=13 INTERNAL value
     * @property {number} UNAVAILABLE=14 UNAVAILABLE value
     * @property {number} DATA_LOSS=15 DATA_LOSS value
     * @property {number} UNAUTHENTICATED=16 UNAUTHENTICATED value
     */
    forpc.StatusCode = (function() {
        var valuesById = {}, values = Object.create(valuesById);
        values[valuesById[0] = "OK"] = 0;
        values[valuesById[1] = "CANCELLED"] = 1;
        values[valuesById[2] = "UNKNOWN"] = 2;
        values[valuesById[3] = "INVALID_ARGUMENT"] = 3;
        values[valuesById[4] = "DEADLINE_EXCEEDED"] = 4;
        values[valuesById[5] = "NOT_FOUND"] = 5;
        values[valuesById[6] = "ALREADY_EXISTS"] = 6;
        values[valuesById[7] = "PERMISSION_DENIED"] = 7;
        values[valuesById[8] = "RESOURCE_EXHAUSTED"] = 8;
        values[valuesById[9] = "FAILED_PRECONDITION"] = 9;
        values[valuesById[10] = "ABORTED"] = 10;
        values[valuesById[11] = "OUT_OF_RANGE"] = 11;
        values[valuesById[12] = "UNIMPLEMENTED"] = 12;
        values[valuesById[13] = "INTERNAL"] = 13;
        values[valuesById[14] = "UNAVAILABLE"] = 14;
        values[valuesById[15] = "DATA_LOSS"] = 15;
        values[valuesById[16] = "UNAUTHENTICATED"] = 16;
        return values;
    })();

    forpc.Call = (function() {

        /**
         * Properties of a Call.
         * @memberof forpc
         * @interface ICall
         * @property {string|null} [method] Call method
         * @property {Object.<string,string>|null} [metadata] Call metadata
         */

        /**
         * Constructs a new Call.
         * @memberof forpc
         * @classdesc Represents a Call.
         * @implements ICall
         * @constructor
         * @param {forpc.ICall=} [properties] Properties to set
         */
        function Call(properties) {
            this.metadata = {};
            if (properties)
                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Call method.
         * @member {string} method
         * @memberof forpc.Call
         * @instance
         */
        Call.prototype.method = "";

        /**
         * Call metadata.
         * @member {Object.<string,string>} metadata
         * @memberof forpc.Call
         * @instance
         */
        Call.prototype.metadata = $util.emptyObject;

        /**
         * Creates a new Call instance using the specified properties.
         * @function create
         * @memberof forpc.Call
         * @static
         * @param {forpc.ICall=} [properties] Properties to set
         * @returns {forpc.Call} Call instance
         */
        Call.create = function create(properties) {
            return new Call(properties);
        };

        /**
         * Encodes the specified Call message. Does not implicitly {@link forpc.Call.verify|verify} messages.
         * @function encode
         * @memberof forpc.Call
         * @static
         * @param {forpc.ICall} message Call message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Call.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.method != null && Object.hasOwnProperty.call(message, "method"))
                writer.uint32(/* id 1, wireType 2 =*/10).string(message.method);
            if (message.metadata != null && Object.hasOwnProperty.call(message, "metadata"))
                for (var keys = Object.keys(message.metadata), i = 0; i < keys.length; ++i)
                    writer.uint32(/* id 2, wireType 2 =*/18).fork().uint32(/* id 1, wireType 2 =*/10).string(keys[i]).uint32(/* id 2, wireType 2 =*/18).string(message.metadata[keys[i]]).ldelim();
            return writer;
        };

        /**
         * Encodes the specified Call message, length delimited. Does not implicitly {@link forpc.Call.verify|verify} messages.
         * @function encodeDelimited
         * @memberof forpc.Call
         * @static
         * @param {forpc.ICall} message Call message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Call.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a Call message from the specified reader or buffer.
         * @function decode
         * @memberof forpc.Call
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {forpc.Call} Call
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Call.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.forpc.Call(), key, value;
            while (reader.pos < end) {
                var tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.method = reader.string();
                        break;
                    }
                case 2: {
                        if (message.metadata === $util.emptyObject)
                            message.metadata = {};
                        var end2 = reader.uint32() + reader.pos;
                        key = "";
                        value = "";
                        while (reader.pos < end2) {
                            var tag2 = reader.uint32();
                            switch (tag2 >>> 3) {
                            case 1:
                                key = reader.string();
                                break;
                            case 2:
                                value = reader.string();
                                break;
                            default:
                                reader.skipType(tag2 & 7);
                                break;
                            }
                        }
                        message.metadata[key] = value;
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Decodes a Call message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof forpc.Call
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {forpc.Call} Call
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Call.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Call message.
         * @function verify
         * @memberof forpc.Call
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Call.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.method != null && message.hasOwnProperty("method"))
                if (!$util.isString(message.method))
                    return "method: string expected";
            if (message.metadata != null && message.hasOwnProperty("metadata")) {
                if (!$util.isObject(message.metadata))
                    return "metadata: object expected";
                var key = Object.keys(message.metadata);
                for (var i = 0; i < key.length; ++i)
                    if (!$util.isString(message.metadata[key[i]]))
                        return "metadata: string{k:string} expected";
            }
            return null;
        };

        /**
         * Creates a Call message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof forpc.Call
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {forpc.Call} Call
         */
        Call.fromObject = function fromObject(object) {
            if (object instanceof $root.forpc.Call)
                return object;
            var message = new $root.forpc.Call();
            if (object.method != null)
                message.method = String(object.method);
            if (object.metadata) {
                if (typeof object.metadata !== "object")
                    throw TypeError(".forpc.Call.metadata: object expected");
                message.metadata = {};
                for (var keys = Object.keys(object.metadata), i = 0; i < keys.length; ++i)
                    message.metadata[keys[i]] = String(object.metadata[keys[i]]);
            }
            return message;
        };

        /**
         * Creates a plain object from a Call message. Also converts values to other types if specified.
         * @function toObject
         * @memberof forpc.Call
         * @static
         * @param {forpc.Call} message Call
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Call.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            var object = {};
            if (options.objects || options.defaults)
                object.metadata = {};
            if (options.defaults)
                object.method = "";
            if (message.method != null && message.hasOwnProperty("method"))
                object.method = message.method;
            var keys2;
            if (message.metadata && (keys2 = Object.keys(message.metadata)).length) {
                object.metadata = {};
                for (var j = 0; j < keys2.length; ++j)
                    object.metadata[keys2[j]] = message.metadata[keys2[j]];
            }
            return object;
        };

        /**
         * Converts this Call to JSON.
         * @function toJSON
         * @memberof forpc.Call
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Call.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Call
         * @function getTypeUrl
         * @memberof forpc.Call
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Call.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/forpc.Call";
        };

        return Call;
    })();

    forpc.Status = (function() {

        /**
         * Properties of a Status.
         * @memberof forpc
         * @interface IStatus
         * @property {forpc.StatusCode|null} [code] Status code
         * @property {string|null} [message] Status message
         */

        /**
         * Constructs a new Status.
         * @memberof forpc
         * @classdesc Represents a Status.
         * @implements IStatus
         * @constructor
         * @param {forpc.IStatus=} [properties] Properties to set
         */
        function Status(properties) {
            if (properties)
                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Status code.
         * @member {forpc.StatusCode} code
         * @memberof forpc.Status
         * @instance
         */
        Status.prototype.code = 0;

        /**
         * Status message.
         * @member {string} message
         * @memberof forpc.Status
         * @instance
         */
        Status.prototype.message = "";

        /**
         * Creates a new Status instance using the specified properties.
         * @function create
         * @memberof forpc.Status
         * @static
         * @param {forpc.IStatus=} [properties] Properties to set
         * @returns {forpc.Status} Status instance
         */
        Status.create = function create(properties) {
            return new Status(properties);
        };

        /**
         * Encodes the specified Status message. Does not implicitly {@link forpc.Status.verify|verify} messages.
         * @function encode
         * @memberof forpc.Status
         * @static
         * @param {forpc.IStatus} message Status message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Status.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.code != null && Object.hasOwnProperty.call(message, "code"))
                writer.uint32(/* id 1, wireType 0 =*/8).int32(message.code);
            if (message.message != null && Object.hasOwnProperty.call(message, "message"))
                writer.uint32(/* id 2, wireType 2 =*/18).string(message.message);
            return writer;
        };

        /**
         * Encodes the specified Status message, length delimited. Does not implicitly {@link forpc.Status.verify|verify} messages.
         * @function encodeDelimited
         * @memberof forpc.Status
         * @static
         * @param {forpc.IStatus} message Status message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Status.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a Status message from the specified reader or buffer.
         * @function decode
         * @memberof forpc.Status
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {forpc.Status} Status
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Status.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.forpc.Status();
            while (reader.pos < end) {
                var tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.code = reader.int32();
                        break;
                    }
                case 2: {
                        message.message = reader.string();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Decodes a Status message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof forpc.Status
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {forpc.Status} Status
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Status.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Status message.
         * @function verify
         * @memberof forpc.Status
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Status.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.code != null && message.hasOwnProperty("code"))
                switch (message.code) {
                default:
                    return "code: enum value expected";
                case 0:
                case 1:
                case 2:
                case 3:
                case 4:
                case 5:
                case 6:
                case 7:
                case 8:
                case 9:
                case 10:
                case 11:
                case 12:
                case 13:
                case 14:
                case 15:
                case 16:
                    break;
                }
            if (message.message != null && message.hasOwnProperty("message"))
                if (!$util.isString(message.message))
                    return "message: string expected";
            return null;
        };

        /**
         * Creates a Status message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof forpc.Status
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {forpc.Status} Status
         */
        Status.fromObject = function fromObject(object) {
            if (object instanceof $root.forpc.Status)
                return object;
            var message = new $root.forpc.Status();
            switch (object.code) {
            default:
                if (typeof object.code === "number") {
                    message.code = object.code;
                    break;
                }
                break;
            case "OK":
            case 0:
                message.code = 0;
                break;
            case "CANCELLED":
            case 1:
                message.code = 1;
                break;
            case "UNKNOWN":
            case 2:
                message.code = 2;
                break;
            case "INVALID_ARGUMENT":
            case 3:
                message.code = 3;
                break;
            case "DEADLINE_EXCEEDED":
            case 4:
                message.code = 4;
                break;
            case "NOT_FOUND":
            case 5:
                message.code = 5;
                break;
            case "ALREADY_EXISTS":
            case 6:
                message.code = 6;
                break;
            case "PERMISSION_DENIED":
            case 7:
                message.code = 7;
                break;
            case "RESOURCE_EXHAUSTED":
            case 8:
                message.code = 8;
                break;
            case "FAILED_PRECONDITION":
            case 9:
                message.code = 9;
                break;
            case "ABORTED":
            case 10:
                message.code = 10;
                break;
            case "OUT_OF_RANGE":
            case 11:
                message.code = 11;
                break;
            case "UNIMPLEMENTED":
            case 12:
                message.code = 12;
                break;
            case "INTERNAL":
            case 13:
                message.code = 13;
                break;
            case "UNAVAILABLE":
            case 14:
                message.code = 14;
                break;
            case "DATA_LOSS":
            case 15:
                message.code = 15;
                break;
            case "UNAUTHENTICATED":
            case 16:
                message.code = 16;
                break;
            }
            if (object.message != null)
                message.message = String(object.message);
            return message;
        };

        /**
         * Creates a plain object from a Status message. Also converts values to other types if specified.
         * @function toObject
         * @memberof forpc.Status
         * @static
         * @param {forpc.Status} message Status
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Status.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            var object = {};
            if (options.defaults) {
                object.code = options.enums === String ? "OK" : 0;
                object.message = "";
            }
            if (message.code != null && message.hasOwnProperty("code"))
                object.code = options.enums === String ? $root.forpc.StatusCode[message.code] === undefined ? message.code : $root.forpc.StatusCode[message.code] : message.code;
            if (message.message != null && message.hasOwnProperty("message"))
                object.message = message.message;
            return object;
        };

        /**
         * Converts this Status to JSON.
         * @function toJSON
         * @memberof forpc.Status
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Status.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Status
         * @function getTypeUrl
         * @memberof forpc.Status
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Status.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/forpc.Status";
        };

        return Status;
    })();

    /**
     * FrameKind enum.
     * @name forpc.FrameKind
     * @enum {number}
     * @property {number} HEADERS=0 HEADERS value
     * @property {number} DATA=1 DATA value
     * @property {number} TRAILERS=2 TRAILERS value
     * @property {number} RST_STREAM=3 RST_STREAM value
     */
    forpc.FrameKind = (function() {
        var valuesById = {}, values = Object.create(valuesById);
        values[valuesById[0] = "HEADERS"] = 0;
        values[valuesById[1] = "DATA"] = 1;
        values[valuesById[2] = "TRAILERS"] = 2;
        values[valuesById[3] = "RST_STREAM"] = 3;
        return values;
    })();

    forpc.Packet = (function() {

        /**
         * Properties of a Packet.
         * @memberof forpc
         * @interface IPacket
         * @property {number|null} [streamId] Packet streamId
         * @property {forpc.FrameKind|null} [kind] Packet kind
         * @property {Uint8Array|null} [payload] Packet payload
         * @property {number|null} [errorCode] Packet errorCode
         */

        /**
         * Constructs a new Packet.
         * @memberof forpc
         * @classdesc Represents a Packet.
         * @implements IPacket
         * @constructor
         * @param {forpc.IPacket=} [properties] Properties to set
         */
        function Packet(properties) {
            if (properties)
                for (var keys = Object.keys(properties), i = 0; i < keys.length; ++i)
                    if (properties[keys[i]] != null)
                        this[keys[i]] = properties[keys[i]];
        }

        /**
         * Packet streamId.
         * @member {number} streamId
         * @memberof forpc.Packet
         * @instance
         */
        Packet.prototype.streamId = 0;

        /**
         * Packet kind.
         * @member {forpc.FrameKind} kind
         * @memberof forpc.Packet
         * @instance
         */
        Packet.prototype.kind = 0;

        /**
         * Packet payload.
         * @member {Uint8Array} payload
         * @memberof forpc.Packet
         * @instance
         */
        Packet.prototype.payload = $util.newBuffer([]);

        /**
         * Packet errorCode.
         * @member {number} errorCode
         * @memberof forpc.Packet
         * @instance
         */
        Packet.prototype.errorCode = 0;

        /**
         * Creates a new Packet instance using the specified properties.
         * @function create
         * @memberof forpc.Packet
         * @static
         * @param {forpc.IPacket=} [properties] Properties to set
         * @returns {forpc.Packet} Packet instance
         */
        Packet.create = function create(properties) {
            return new Packet(properties);
        };

        /**
         * Encodes the specified Packet message. Does not implicitly {@link forpc.Packet.verify|verify} messages.
         * @function encode
         * @memberof forpc.Packet
         * @static
         * @param {forpc.IPacket} message Packet message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Packet.encode = function encode(message, writer) {
            if (!writer)
                writer = $Writer.create();
            if (message.streamId != null && Object.hasOwnProperty.call(message, "streamId"))
                writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.streamId);
            if (message.kind != null && Object.hasOwnProperty.call(message, "kind"))
                writer.uint32(/* id 2, wireType 0 =*/16).int32(message.kind);
            if (message.payload != null && Object.hasOwnProperty.call(message, "payload"))
                writer.uint32(/* id 3, wireType 2 =*/26).bytes(message.payload);
            if (message.errorCode != null && Object.hasOwnProperty.call(message, "errorCode"))
                writer.uint32(/* id 4, wireType 0 =*/32).uint32(message.errorCode);
            return writer;
        };

        /**
         * Encodes the specified Packet message, length delimited. Does not implicitly {@link forpc.Packet.verify|verify} messages.
         * @function encodeDelimited
         * @memberof forpc.Packet
         * @static
         * @param {forpc.IPacket} message Packet message or plain object to encode
         * @param {$protobuf.Writer} [writer] Writer to encode to
         * @returns {$protobuf.Writer} Writer
         */
        Packet.encodeDelimited = function encodeDelimited(message, writer) {
            return this.encode(message, writer).ldelim();
        };

        /**
         * Decodes a Packet message from the specified reader or buffer.
         * @function decode
         * @memberof forpc.Packet
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @param {number} [length] Message length if known beforehand
         * @returns {forpc.Packet} Packet
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Packet.decode = function decode(reader, length, error) {
            if (!(reader instanceof $Reader))
                reader = $Reader.create(reader);
            var end = length === undefined ? reader.len : reader.pos + length, message = new $root.forpc.Packet();
            while (reader.pos < end) {
                var tag = reader.uint32();
                if (tag === error)
                    break;
                switch (tag >>> 3) {
                case 1: {
                        message.streamId = reader.uint32();
                        break;
                    }
                case 2: {
                        message.kind = reader.int32();
                        break;
                    }
                case 3: {
                        message.payload = reader.bytes();
                        break;
                    }
                case 4: {
                        message.errorCode = reader.uint32();
                        break;
                    }
                default:
                    reader.skipType(tag & 7);
                    break;
                }
            }
            return message;
        };

        /**
         * Decodes a Packet message from the specified reader or buffer, length delimited.
         * @function decodeDelimited
         * @memberof forpc.Packet
         * @static
         * @param {$protobuf.Reader|Uint8Array} reader Reader or buffer to decode from
         * @returns {forpc.Packet} Packet
         * @throws {Error} If the payload is not a reader or valid buffer
         * @throws {$protobuf.util.ProtocolError} If required fields are missing
         */
        Packet.decodeDelimited = function decodeDelimited(reader) {
            if (!(reader instanceof $Reader))
                reader = new $Reader(reader);
            return this.decode(reader, reader.uint32());
        };

        /**
         * Verifies a Packet message.
         * @function verify
         * @memberof forpc.Packet
         * @static
         * @param {Object.<string,*>} message Plain object to verify
         * @returns {string|null} `null` if valid, otherwise the reason why it is not
         */
        Packet.verify = function verify(message) {
            if (typeof message !== "object" || message === null)
                return "object expected";
            if (message.streamId != null && message.hasOwnProperty("streamId"))
                if (!$util.isInteger(message.streamId))
                    return "streamId: integer expected";
            if (message.kind != null && message.hasOwnProperty("kind"))
                switch (message.kind) {
                default:
                    return "kind: enum value expected";
                case 0:
                case 1:
                case 2:
                case 3:
                    break;
                }
            if (message.payload != null && message.hasOwnProperty("payload"))
                if (!(message.payload && typeof message.payload.length === "number" || $util.isString(message.payload)))
                    return "payload: buffer expected";
            if (message.errorCode != null && message.hasOwnProperty("errorCode"))
                if (!$util.isInteger(message.errorCode))
                    return "errorCode: integer expected";
            return null;
        };

        /**
         * Creates a Packet message from a plain object. Also converts values to their respective internal types.
         * @function fromObject
         * @memberof forpc.Packet
         * @static
         * @param {Object.<string,*>} object Plain object
         * @returns {forpc.Packet} Packet
         */
        Packet.fromObject = function fromObject(object) {
            if (object instanceof $root.forpc.Packet)
                return object;
            var message = new $root.forpc.Packet();
            if (object.streamId != null)
                message.streamId = object.streamId >>> 0;
            switch (object.kind) {
            default:
                if (typeof object.kind === "number") {
                    message.kind = object.kind;
                    break;
                }
                break;
            case "HEADERS":
            case 0:
                message.kind = 0;
                break;
            case "DATA":
            case 1:
                message.kind = 1;
                break;
            case "TRAILERS":
            case 2:
                message.kind = 2;
                break;
            case "RST_STREAM":
            case 3:
                message.kind = 3;
                break;
            }
            if (object.payload != null)
                if (typeof object.payload === "string")
                    $util.base64.decode(object.payload, message.payload = $util.newBuffer($util.base64.length(object.payload)), 0);
                else if (object.payload.length >= 0)
                    message.payload = object.payload;
            if (object.errorCode != null)
                message.errorCode = object.errorCode >>> 0;
            return message;
        };

        /**
         * Creates a plain object from a Packet message. Also converts values to other types if specified.
         * @function toObject
         * @memberof forpc.Packet
         * @static
         * @param {forpc.Packet} message Packet
         * @param {$protobuf.IConversionOptions} [options] Conversion options
         * @returns {Object.<string,*>} Plain object
         */
        Packet.toObject = function toObject(message, options) {
            if (!options)
                options = {};
            var object = {};
            if (options.defaults) {
                object.streamId = 0;
                object.kind = options.enums === String ? "HEADERS" : 0;
                if (options.bytes === String)
                    object.payload = "";
                else {
                    object.payload = [];
                    if (options.bytes !== Array)
                        object.payload = $util.newBuffer(object.payload);
                }
                object.errorCode = 0;
            }
            if (message.streamId != null && message.hasOwnProperty("streamId"))
                object.streamId = message.streamId;
            if (message.kind != null && message.hasOwnProperty("kind"))
                object.kind = options.enums === String ? $root.forpc.FrameKind[message.kind] === undefined ? message.kind : $root.forpc.FrameKind[message.kind] : message.kind;
            if (message.payload != null && message.hasOwnProperty("payload"))
                object.payload = options.bytes === String ? $util.base64.encode(message.payload, 0, message.payload.length) : options.bytes === Array ? Array.prototype.slice.call(message.payload) : message.payload;
            if (message.errorCode != null && message.hasOwnProperty("errorCode"))
                object.errorCode = message.errorCode;
            return object;
        };

        /**
         * Converts this Packet to JSON.
         * @function toJSON
         * @memberof forpc.Packet
         * @instance
         * @returns {Object.<string,*>} JSON object
         */
        Packet.prototype.toJSON = function toJSON() {
            return this.constructor.toObject(this, $protobuf.util.toJSONOptions);
        };

        /**
         * Gets the default type url for Packet
         * @function getTypeUrl
         * @memberof forpc.Packet
         * @static
         * @param {string} [typeUrlPrefix] your custom typeUrlPrefix(default "type.googleapis.com")
         * @returns {string} The default type url
         */
        Packet.getTypeUrl = function getTypeUrl(typeUrlPrefix) {
            if (typeUrlPrefix === undefined) {
                typeUrlPrefix = "type.googleapis.com";
            }
            return typeUrlPrefix + "/forpc.Packet";
        };

        return Packet;
    })();

    return forpc;
})();

module.exports = $root;
