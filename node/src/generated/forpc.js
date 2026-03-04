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
         * @property {number|null} [code] Status code
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
         * @member {number} code
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
                writer.uint32(/* id 1, wireType 0 =*/8).uint32(message.code);
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
                        message.code = reader.uint32();
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
                if (!$util.isInteger(message.code))
                    return "code: integer expected";
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
            if (object.code != null)
                message.code = object.code >>> 0;
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
                object.code = 0;
                object.message = "";
            }
            if (message.code != null && message.hasOwnProperty("code"))
                object.code = message.code;
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

    return forpc;
})();

module.exports = $root;
