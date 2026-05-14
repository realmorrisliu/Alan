#if os(macOS)
import Darwin

enum AlanMacAppStartup {
    static func acquireSingletonOrTerminate() -> AlanAppSingletonGuard {
        do {
            switch try AlanAppSingletonGuard.acquire() {
            case .acquired(let guardHandle):
                return guardHandle
            case .alreadyRunning:
                AlanAppSingletonGuard.activateExistingInstance()
                Darwin.exit(0)
            }
        } catch {
            fatalError("alan could not acquire the macOS app singleton lock: \(error)")
        }
    }
}
#endif
