// Lists on-screen windows front to back with owner, layer and bounds, using the public
// CGWindowListCopyWindowInfo. Used to confirm the Prompter panel (layer 1000) sits above a
// full screen app's window without touching the screen. Build: see README.md.
#import <CoreGraphics/CoreGraphics.h>
#import <Foundation/Foundation.h>

int main(int argc, char **argv) {
    @autoreleasepool {
        CFArrayRef list = CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID);
        NSArray *windows = CFBridgingRelease(list);
        BOOL onlyAside = argc > 1 && strcmp(argv[1], "--aside") == 0;
        NSUInteger order = 0;
        for (NSDictionary *info in windows) {
            NSString *owner = info[(id)kCGWindowOwnerName] ?: @"?";
            NSNumber *layer = info[(id)kCGWindowLayer];
            NSNumber *number = info[(id)kCGWindowNumber];
            NSNumber *alpha = info[(id)kCGWindowAlpha];
            NSString *name = info[(id)kCGWindowName] ?: @"";
            NSDictionary *bounds = info[(id)kCGWindowBounds];
            BOOL isAside = [owner isEqualToString:@"aside"] || [owner isEqualToString:@"Aside"];
            if (onlyAside && !isAside) { order += 1; continue; }
            printf("%3lu  layer %5ld  alpha %.2f  #%-6ld %-24s %-28s x=%g y=%g w=%g h=%g\n",
                   (unsigned long)order, (long)layer.integerValue, alpha.doubleValue, (long)number.integerValue,
                   owner.UTF8String, name.UTF8String,
                   [bounds[@"X"] doubleValue], [bounds[@"Y"] doubleValue], [bounds[@"Width"] doubleValue], [bounds[@"Height"] doubleValue]);
            order += 1;
        }
    }
    return 0;
}
