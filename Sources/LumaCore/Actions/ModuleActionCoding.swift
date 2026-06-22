import Foundation

public enum ModuleActionCoding {
    public static func encode<T: Encodable>(_ action: T) throws -> Data {
        try JSONEncoder().encode(action)
    }

    public static func decode<T: Decodable>(_ type: T.Type, from data: Data) throws -> T {
        try JSONDecoder().decode(type, from: data)
    }
}
