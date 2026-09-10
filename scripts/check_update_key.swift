// Validate only the public identity of a CI seed. Never print key material.
import CryptoKit
import Foundation

let input = FileHandle.standardInput.readDataToEndOfFile()
guard CommandLine.arguments.count == 2,
      let text = String(data: input, encoding: .utf8),
      let seed = Data(base64Encoded: text.trimmingCharacters(in: .whitespacesAndNewlines)),
      seed.count == 32,
      let key = try? Curve25519.Signing.PrivateKey(rawRepresentation: seed),
      key.publicKey.rawRepresentation.base64EncodedString() == CommandLine.arguments[1]
else { exit(1) }
