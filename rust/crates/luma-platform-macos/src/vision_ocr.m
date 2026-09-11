#import <Foundation/Foundation.h>
#import <Vision/Vision.h>
#include <stdlib.h>
#include <string.h>

// Synchronous, platform-local Vision boundary.
// Returns 0 with an allocated UTF-8 string, 1 for no text, and 2 for recognition failure.
int luma_vision_recognize(const char *image_path, char **out_text) {
    if (image_path == NULL || out_text == NULL) {
        return 2;
    }
    *out_text = NULL;
    @autoreleasepool {
        @try {
            NSString *path = [NSString stringWithUTF8String:image_path];
            if (path == nil) {
                return 2;
            }
            VNRecognizeTextRequest *request = [[VNRecognizeTextRequest alloc] init];
            request.recognitionLevel = VNRequestTextRecognitionLevelAccurate;
            request.usesLanguageCorrection = YES;
            request.recognitionLanguages = @[@"zh-Hans", @"zh-Hant", @"en-US"];

            VNImageRequestHandler *handler = [[VNImageRequestHandler alloc]
                initWithURL:[NSURL fileURLWithPath:path]
                    options:@{}];
            NSError *error = nil;
            if ([handler performRequests:@[request] error:&error] == NO) {
                return 2;
            }
            NSArray<VNRecognizedTextObservation *> *observations =
                [request.results sortedArrayUsingComparator:^NSComparisonResult(
                    VNRecognizedTextObservation *left,
                    VNRecognizedTextObservation *right) {
                    CGRect l = left.boundingBox;
                    CGRect r = right.boundingBox;
                    if (fabs(CGRectGetMaxY(l) - CGRectGetMaxY(r)) > 0.005) {
                        return CGRectGetMaxY(l) > CGRectGetMaxY(r)
                            ? NSOrderedAscending
                            : NSOrderedDescending;
                    }
                    if (CGRectGetMinX(l) < CGRectGetMinX(r)) {
                        return NSOrderedAscending;
                    }
                    if (CGRectGetMinX(l) > CGRectGetMinX(r)) {
                        return NSOrderedDescending;
                    }
                    return NSOrderedSame;
                }];
            NSMutableString *recognized = [NSMutableString string];
            for (VNRecognizedTextObservation *observation in observations) {
                VNRecognizedText *candidate = [[observation topCandidates:1] firstObject];
                if (candidate == nil || candidate.string.length == 0) {
                    continue;
                }
                if (recognized.length > 0) {
                    [recognized appendString:@"\n"];
                }
                [recognized appendString:candidate.string];
            }
            if (recognized.length == 0) {
                return 1;
            }
            const char *utf8 = recognized.UTF8String;
            if (utf8 == NULL) {
                return 2;
            }
            *out_text = strdup(utf8);
            return *out_text == NULL ? 2 : 0;
        } @catch (NSException *exception) {
            return 2;
        }
    }
}

void luma_vision_free(char *text) {
    free(text);
}
